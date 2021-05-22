#![allow(clippy::field_reassign_with_default)]

// from https://github1s.com/Fluidex/circuits/blob/HEAD/test/global_state.ts

use super::global::GlobalState;
use crate::types::l2::{tx_detail_idx, DepositToOldTx, L2Block, Order, RawTx, SpotTradeTx, TxType, TX_LENGTH};
use crate::types::merkle_tree::Tree;
use crate::types::primitives::{bigint_to_fr, field_to_bigint, u32_to_fr, Fr};
use ff::Field;

// TODO: change to snake_case
// TODO: too many unwrap here
pub struct WitnessGenerator {
    state: GlobalState,
    n_tx: usize,
    buffered_txs: Vec<RawTx>,
    buffered_blocks: Vec<L2Block>,
    verbose: bool,
}

impl WitnessGenerator {
    pub fn print_config() {
        Tree::print_config();
    }
    pub fn new(state: GlobalState, n_tx: usize, verbose: bool) -> Self {
        Self {
            state,
            n_tx,
            buffered_txs: Vec::new(),
            buffered_blocks: Vec::new(),
            verbose,
        }
    }
    pub fn root(&self) -> Fr {
        self.state.root()
    }
    pub fn has_order(&self, account_id: u32, order_id: u32) -> bool {
        self.state.has_order(account_id, order_id)
    }
    pub fn get_token_balance(&self, account_id: u32, token_id: u32) -> Fr {
        self.state.get_token_balance(account_id, token_id)
    }
    pub fn update_order_state(&mut self, account_id: u32, order: Order) {
        self.state.update_order_state(account_id, order)
    }
    pub fn create_new_account(&mut self, next_order_id: u32) -> u32 {
        self.state.create_new_account(next_order_id)
    }
    pub fn get_account_order_by_id(&self, account_id: u32, order_id: u32) -> Order {
        self.state.get_account_order_by_id(account_id, order_id)
    }
    pub fn forge_with_txs(&self, buffered_txs: &[RawTx]) -> L2Block {
        assert!(buffered_txs.len() == self.n_tx, "invalid txs len");
        let txs_type = buffered_txs.iter().map(|tx| tx.tx_type).collect();
        let encoded_txs = buffered_txs.iter().map(|tx| tx.payload.clone()).collect();
        let balance_path_elements = buffered_txs
            .iter()
            .map(|tx| {
                [
                    tx.balance_path0.clone(),
                    tx.balance_path1.clone(),
                    tx.balance_path2.clone(),
                    tx.balance_path3.clone(),
                ]
            })
            .collect();
        let order_path_elements = buffered_txs
            .iter()
            .map(|tx| [tx.order_path0.clone(), tx.order_path1.clone()])
            .collect();
        let order_roots = buffered_txs.iter().map(|tx| [tx.order_root0, tx.order_root1]).collect();
        let account_path_elements = buffered_txs
            .iter()
            .map(|tx| [tx.account_path0.clone(), tx.account_path1.clone()])
            .collect();
        let old_account_roots = buffered_txs.iter().map(|tx| tx.root_before).collect();
        let new_account_roots = buffered_txs.iter().map(|tx| tx.root_after).collect();
        L2Block {
            txs_type,
            encoded_txs,
            balance_path_elements,
            order_path_elements,
            account_path_elements,
            order_roots,
            old_account_roots,
            new_account_roots,
        }
    }
    pub fn forge(&mut self) -> L2Block {
        self.flush_with_nop();
        self.forge_with_txs(&self.buffered_txs)
    }
    pub fn forge_all_l2_blocks(&mut self) -> Vec<L2Block> {
        self.buffered_blocks.clone()
    }
    pub fn add_raw_tx(&mut self, raw_tx: RawTx) {
        self.buffered_txs.push(raw_tx);
        if self.buffered_txs.len() % self.n_tx == 0 {
            // forge next block, using last n_tx txs
            let txs = &self.buffered_txs[(self.buffered_txs.len() - self.n_tx)..];
            let block = self.forge_with_txs(txs);
            self.buffered_blocks.push(block);
            assert!(
                self.buffered_blocks.len() * self.n_tx == self.buffered_txs.len(),
                "invalid block num"
            );
            if self.verbose {
                println!("forge block {} done", self.buffered_blocks.len() - 1);
            }
        }
    }
    pub fn get_buffered_blocks(&self) -> &[L2Block] {
        self.buffered_blocks.as_slice()
    }
    pub fn take_blocks(self) -> Vec<L2Block> {
        self.buffered_blocks
    }

    /*
    DepositToNew(tx: DepositToNewTx) {
      assert!(self.accounts.get(tx.account_id).eth_addr == 0n, "DepositToNew");
      let proof = self.state_proof(tx.account_id, tx.token_id);
      // first, generate the tx
      let encoded_tx: Array<Fr> = new Array(Txlen());
      encoded_tx.fill(0n, 0, Txlen());
      encoded_tx[tx_detail_idx::TOKEN_ID] = Scalar.e(tx.token_id);
      encoded_tx[tx_detail_idx::AMOUNT] = tx.amount;
      encoded_tx[tx_detail_idx::ACCOUNT_ID2] = Scalar.e(tx.account_id);
      encoded_tx[tx_detail_idx::ETH_ADDR2] = tx.eth_addr;
      encoded_tx[tx_detail_idx::SIGN2] = Scalar.e(tx.sign);
      encoded_tx[tx_detail_idx::AY2] = tx.ay;
      let raw_tx: RawTx = {
        tx_type: TxType.DepositToNew,
        payload: encoded_tx,
        balance_path0: proof.balance_path,
        balance_path1: proof.balance_path,
        balance_path2: proof.balance_path,
        balance_path3: proof.balance_path,
        order_path0: self.trivial_order_path_elements(),
        order_path1: self.trivial_order_path_elements(),
        order_root0: self.default_order_root,
        order_root1: self.default_order_root,
        account_path0: proof.account_path,
        account_path1: proof.account_path,
        root_before: proof.root,
        root_after: 0n,
      };

      // then update global state
      self.set_token_balance(tx.account_id, tx.token_id, tx.amount);
      self.setAccountL2Addr(tx.account_id, tx.sign, tx.ay, tx.eth_addr);
      raw_tx.root_after = self.root();
      self.add_raw_tx(raw_tx);
    }
    */
    pub fn deposit_to_old(&mut self, tx: DepositToOldTx) {
        //assert!(self.accounts.get(tx.account_id).eth_addr != 0n, "deposit_to_old");
        let proof = self.state.state_proof(tx.account_id, tx.token_id);
        let old_balance = self.state.get_token_balance(tx.account_id, tx.token_id);
        let acc = self.state.get_account(tx.account_id);
        let nonce = acc.nonce;
        // first, generate the tx

        let mut encoded_tx = [Fr::zero(); TX_LENGTH];
        encoded_tx[tx_detail_idx::AMOUNT] = tx.amount;

        encoded_tx[tx_detail_idx::TOKEN_ID1] = u32_to_fr(tx.token_id);
        encoded_tx[tx_detail_idx::ACCOUNT_ID1] = u32_to_fr(tx.account_id);
        encoded_tx[tx_detail_idx::BALANCE1] = old_balance;
        encoded_tx[tx_detail_idx::NONCE1] = nonce;
        encoded_tx[tx_detail_idx::ETH_ADDR1] = acc.eth_addr;
        encoded_tx[tx_detail_idx::SIGN1] = acc.sign;
        encoded_tx[tx_detail_idx::AY1] = acc.ay;

        encoded_tx[tx_detail_idx::TOKEN_ID2] = u32_to_fr(tx.token_id);
        encoded_tx[tx_detail_idx::ACCOUNT_ID2] = u32_to_fr(tx.account_id);
        encoded_tx[tx_detail_idx::BALANCE2] = bigint_to_fr(field_to_bigint(&old_balance) + field_to_bigint(&tx.amount));
        encoded_tx[tx_detail_idx::NONCE2] = nonce;
        encoded_tx[tx_detail_idx::ETH_ADDR2] = acc.eth_addr;
        encoded_tx[tx_detail_idx::SIGN2] = acc.sign;
        encoded_tx[tx_detail_idx::AY2] = acc.ay;

        encoded_tx[tx_detail_idx::ENABLE_BALANCE_CHECK1] = u32_to_fr(1u32);
        encoded_tx[tx_detail_idx::ENABLE_BALANCE_CHECK2] = u32_to_fr(1u32);

        let mut raw_tx = RawTx {
            tx_type: TxType::DepositToOld,
            payload: encoded_tx.to_vec(),
            balance_path0: proof.balance_path.clone(),
            balance_path1: proof.balance_path.clone(),
            balance_path2: proof.balance_path.clone(),
            balance_path3: proof.balance_path,
            order_path0: self.state.trivial_order_path_elements(),
            order_path1: self.state.trivial_order_path_elements(),
            order_root0: acc.order_root,
            order_root1: acc.order_root,
            account_path0: proof.account_path.clone(),
            account_path1: proof.account_path,
            root_before: proof.root,
            root_after: Fr::zero(),
        };

        let mut balance = old_balance;
        balance.add_assign(&tx.amount);
        self.state.set_token_balance(tx.account_id, tx.token_id, balance);

        raw_tx.root_after = self.state.root();
        self.add_raw_tx(raw_tx);
    }
    /*
    fillTransferTx(tx: TranferTx) {
      let fullTx = {
        from: tx.from,
        to: tx.to,
        token_id: tx.token_id,
        amount: tx.amount,
        fromNonce: self.accounts.get(tx.from).nonce,
        toNonce: self.accounts.get(tx.to).nonce,
        old_balanceFrom: self.get_token_balance(tx.from, tx.token_id),
        old_balanceTo: self.get_token_balance(tx.to, tx.token_id),
      };
      return fullTx;
    }
    fillWithdraw_tx(tx: Withdraw_tx) {
      let fullTx = {
        account_id: tx.account_id,
        token_id: tx.token_id,
        amount: tx.amount,
        nonce: self.accounts.get(tx.account_id).nonce,
        old_balance: self.get_token_balance(tx.account_id, tx.token_id),
      };
      return fullTx;
    }
    Transfer(tx: TranferTx) {
      assert!(self.accounts.get(tx.from).eth_addr != 0n, "TransferTx: empty fromAccount");
      assert!(self.accounts.get(tx.to).eth_addr != 0n, "Transfer: empty toAccount");
      let proofFrom = self.state_proof(tx.from, tx.token_id);
      let fromAccount = self.accounts.get(tx.from);
      let toAccount = self.accounts.get(tx.to);

      // first, generate the tx
      let encoded_tx: Array<Fr> = new Array(Txlen());
      encoded_tx.fill(0n, 0, Txlen());

      let fromOldBalance = self.get_token_balance(tx.from, tx.token_id);
      let toOldBalance = self.get_token_balance(tx.to, tx.token_id);
      assert!(fromOldBalance > tx.amount, "Transfer balance not enough");
      encoded_tx[tx_detail_idx::ACCOUNT_ID1] = tx.from;
      encoded_tx[tx_detail_idx::ACCOUNT_ID2] = tx.to;
      encoded_tx[tx_detail_idx::TOKEN_ID] = tx.token_id;
      encoded_tx[tx_detail_idx::AMOUNT] = tx.amount;
      encoded_tx[tx_detail_idx::NONCE1] = fromAccount.nonce;
      encoded_tx[tx_detail_idx::NONCE2] = toAccount.nonce;
      encoded_tx[tx_detail_idx::SIGN1] = fromAccount.sign;
      encoded_tx[tx_detail_idx::SIGN2] = toAccount.sign;
      encoded_tx[tx_detail_idx::AY1] = fromAccount.ay;
      encoded_tx[tx_detail_idx::AY2] = toAccount.ay;
      encoded_tx[tx_detail_idx::ETH_ADDR1] = fromAccount.eth_addr;
      encoded_tx[tx_detail_idx::ETH_ADDR2] = toAccount.eth_addr;
      encoded_tx[tx_detail_idx::BALANCE1] = fromOldBalance;
      encoded_tx[tx_detail_idx::BALANCE2] = toOldBalance;
      encoded_tx[tx_detail_idx::SIG_L2_HASH] = tx.signature.hash;
      encoded_tx[tx_detail_idx::S] = tx.signature.S;
      encoded_tx[tx_detail_idx::R8X] = tx.signature.R8x;
      encoded_tx[tx_detail_idx::R8Y] = tx.signature.R8y;

      let raw_tx: RawTx = {
        tx_type: TxType.Transfer,
        payload: encoded_tx,
        balance_path0: proofFrom.balance_path,
        balance_path1: null,
        balance_path2: proofFrom.balance_path,
        balance_path3: null,
        order_path0: self.trivial_order_path_elements(),
        order_path1: self.trivial_order_path_elements(),
        order_root0: fromAccount.order_root,
        order_root1: toAccount.order_root,
        account_path0: proofFrom.account_path,
        account_path1: null,
        root_before: proofFrom.root,
        root_after: 0n,
      };

      self.set_token_balance(tx.from, tx.token_id, fromOldBalance - tx.amount);
      self.increase_nonce(tx.from);

      let proofTo = self.state_proof(tx.to, tx.token_id);
      raw_tx.balance_path1 = proofTo.balance_path;
      raw_tx.balance_path3 = proofTo.balance_path;
      raw_tx.account_path1 = proofTo.account_path;
      self.set_token_balance(tx.to, tx.token_id, toOldBalance + tx.amount);

      raw_tx.root_after = self.root();
      self.add_raw_tx(raw_tx);
    }
    Withdraw(tx: Withdraw_tx) {
      assert!(self.accounts.get(tx.account_id).eth_addr != 0n, "Withdraw");
      let proof = self.state_proof(tx.account_id, tx.token_id);
      // first, generate the tx
      let encoded_tx: Array<Fr> = new Array(Txlen());
      encoded_tx.fill(0n, 0, Txlen());

      let acc = self.accounts.get(tx.account_id);
      let balanceBefore = self.get_token_balance(tx.account_id, tx.token_id);
      assert!(balanceBefore > tx.amount, "Withdraw balance");
      encoded_tx[tx_detail_idx::ACCOUNT_ID1] = tx.account_id;
      encoded_tx[tx_detail_idx::TOKEN_ID] = tx.token_id;
      encoded_tx[tx_detail_idx::AMOUNT] = tx.amount;
      encoded_tx[tx_detail_idx::NONCE1] = acc.nonce;
      encoded_tx[tx_detail_idx::SIGN1] = acc.sign;
      encoded_tx[tx_detail_idx::AY1] = acc.ay;
      encoded_tx[tx_detail_idx::ETH_ADDR1] = acc.eth_addr;
      encoded_tx[tx_detail_idx::BALANCE1] = balanceBefore;

      encoded_tx[tx_detail_idx::SIG_L2_HASH] = tx.signature.hash;
      encoded_tx[tx_detail_idx::S] = tx.signature.S;
      encoded_tx[tx_detail_idx::R8X] = tx.signature.R8x;
      encoded_tx[tx_detail_idx::R8Y] = tx.signature.R8y;

      let raw_tx: RawTx = {
        tx_type: TxType.Withdraw,
        payload: encoded_tx,
        balance_path0: proof.balance_path,
        balance_path1: proof.balance_path,
        balance_path2: proof.balance_path,
        balance_path3: proof.balance_path,
        order_path0: self.trivial_order_path_elements(),
        order_path1: self.trivial_order_path_elements(),
        order_root0: acc.order_root,
        order_root1: acc.order_root,
        account_path0: proof.account_path,
        account_path1: proof.account_path,
        root_before: proof.root,
        root_after: 0n,
      };

      self.set_token_balance(tx.account_id, tx.token_id, balanceBefore - tx.amount);
      self.increase_nonce(tx.account_id);

      raw_tx.root_after = self.root();
      self.add_raw_tx(raw_tx);
    }
    */

    // case1: old order is empty
    // case2: old order is valid old order with different order id, but we will replace it.
    // case3: old order has same order id, we will modify it
    pub fn spot_trade(&mut self, tx: SpotTradeTx) {
        //assert!(self.accounts.get(tx.order1_account_id).eth_addr != 0n, "SpotTrade account1");
        //assert!(self.accounts.get(tx.order2_account_id).eth_addr != 0n, "SpotTrade account2");

        if tx.order1_account_id == tx.order2_account_id {
            panic!("self trade no allowed");
        }
        assert!(self.state.has_order(tx.order1_account_id, tx.order1_id), "unknown order1");
        assert!(self.state.has_order(tx.order2_account_id, tx.order2_id), "unknown order2");

        let old_root = self.state.root();

        let account1 = self.state.get_account(tx.order1_account_id);
        let order_root0 = account1.order_root;
        let account2 = self.state.get_account(tx.order2_account_id);
        let proof_order1_seller = self.state.state_proof(tx.order1_account_id, tx.token_id_1to2);
        let proof_order2_seller = self.state.state_proof(tx.order2_account_id, tx.token_id_2to1);

        // old_order1 is same as old_order1_in_tree when case3
        // not same when case1 and case2
        let old_order1 = self.state.get_account_order_by_id(tx.order1_account_id, tx.order1_id);
        let old_order2 = self.state.get_account_order_by_id(tx.order2_account_id, tx.order2_id);
        let old_order1_in_tree = self.state.place_order_into_tree(tx.order1_account_id, tx.order1_id);
        let old_order2_in_tree = self.state.place_order_into_tree(tx.order2_account_id, tx.order2_id);

        let order1_pos = self.state.get_order_pos_by_id(tx.order1_account_id, tx.order1_id);
        let order2_pos = self.state.get_order_pos_by_id(tx.order2_account_id, tx.order2_id);

        // first, generate the tx

        let mut encoded_tx = [Fr::zero(); TX_LENGTH];
        encoded_tx[tx_detail_idx::ACCOUNT_ID1] = u32_to_fr(tx.order1_account_id);
        encoded_tx[tx_detail_idx::ACCOUNT_ID2] = u32_to_fr(tx.order2_account_id);
        encoded_tx[tx_detail_idx::ETH_ADDR1] = account1.eth_addr;
        encoded_tx[tx_detail_idx::ETH_ADDR2] = account2.eth_addr;
        encoded_tx[tx_detail_idx::SIGN1] = account1.sign;
        encoded_tx[tx_detail_idx::SIGN2] = account2.sign;
        encoded_tx[tx_detail_idx::AY1] = account1.ay;
        encoded_tx[tx_detail_idx::AY2] = account2.ay;
        encoded_tx[tx_detail_idx::NONCE1] = account1.nonce;
        encoded_tx[tx_detail_idx::NONCE2] = account2.nonce;
        let account1_balance_sell = self.state.get_token_balance(tx.order1_account_id, tx.token_id_1to2);
        let account2_balance_buy = self.state.get_token_balance(tx.order2_account_id, tx.token_id_1to2);
        let account2_balance_sell = self.state.get_token_balance(tx.order2_account_id, tx.token_id_2to1);
        let account1_balance_buy = self.state.get_token_balance(tx.order1_account_id, tx.token_id_2to1);
        assert!(account1_balance_sell > tx.amount_1to2, "balance_1to2");
        assert!(account2_balance_sell > tx.amount_2to1, "balance_2to1");

        encoded_tx[tx_detail_idx::OLD_ORDER1_ID] = old_order1_in_tree.order_id;
        encoded_tx[tx_detail_idx::OLD_ORDER1_TOKEN_SELL] = old_order1_in_tree.tokensell;
        encoded_tx[tx_detail_idx::OLD_ORDER1_FILLED_SELL] = old_order1_in_tree.filled_sell;
        encoded_tx[tx_detail_idx::OLD_ORDER1_AMOUNT_SELL] = old_order1_in_tree.total_sell;
        encoded_tx[tx_detail_idx::OLD_ORDER1_TOKEN_BUY] = old_order1_in_tree.tokenbuy;
        encoded_tx[tx_detail_idx::OLD_ORDER1_FILLED_BUY] = old_order1_in_tree.filled_buy;
        encoded_tx[tx_detail_idx::OLD_ORDER1_AMOUNT_BUY] = old_order1_in_tree.total_buy;

        encoded_tx[tx_detail_idx::OLD_ORDER2_ID] = old_order2_in_tree.order_id;
        encoded_tx[tx_detail_idx::OLD_ORDER2_TOKEN_SELL] = old_order2_in_tree.tokensell;
        encoded_tx[tx_detail_idx::OLD_ORDER2_FILLED_SELL] = old_order2_in_tree.filled_sell;
        encoded_tx[tx_detail_idx::OLD_ORDER2_AMOUNT_SELL] = old_order2_in_tree.total_sell;
        encoded_tx[tx_detail_idx::OLD_ORDER2_TOKEN_BUY] = old_order2_in_tree.tokenbuy;
        encoded_tx[tx_detail_idx::OLD_ORDER2_FILLED_BUY] = old_order2_in_tree.filled_buy;
        encoded_tx[tx_detail_idx::OLD_ORDER2_AMOUNT_BUY] = old_order2_in_tree.total_buy;

        encoded_tx[tx_detail_idx::AMOUNT] = tx.amount_1to2;
        encoded_tx[tx_detail_idx::AMOUNT2] = tx.amount_2to1;
        encoded_tx[tx_detail_idx::ORDER1_POS] = u32_to_fr(order1_pos);
        encoded_tx[tx_detail_idx::ORDER2_POS] = u32_to_fr(order2_pos);

        encoded_tx[tx_detail_idx::BALANCE1] = account1_balance_sell;
        encoded_tx[tx_detail_idx::BALANCE2] = bigint_to_fr(field_to_bigint(&account2_balance_buy) + field_to_bigint(&tx.amount_1to2));
        encoded_tx[tx_detail_idx::BALANCE3] = account2_balance_sell;
        encoded_tx[tx_detail_idx::BALANCE4] = bigint_to_fr(field_to_bigint(&account1_balance_buy) + field_to_bigint(&tx.amount_2to1));

        encoded_tx[tx_detail_idx::ENABLE_BALANCE_CHECK1] = u32_to_fr(1u32);
        encoded_tx[tx_detail_idx::ENABLE_BALANCE_CHECK2] = u32_to_fr(1u32);

        let mut raw_tx = RawTx {
            tx_type: TxType::SpotTrade,
            payload: Vec::default(),
            balance_path0: proof_order1_seller.balance_path,
            balance_path1: Default::default(),
            balance_path2: proof_order2_seller.balance_path,
            balance_path3: Default::default(),
            order_path0: self.state.order_proof(tx.order1_account_id, order1_pos).path_elements,
            order_path1: self.state.order_proof(tx.order2_account_id, order2_pos).path_elements,
            order_root0,
            order_root1: Default::default(),
            account_path0: proof_order1_seller.account_path,
            account_path1: Default::default(),
            root_before: old_root,
            root_after: Default::default(),
        };

        // do not update state root
        // account1 after sending, before receiving
        let mut balance1 = account1_balance_sell;
        balance1.sub_assign(&tx.amount_1to2);
        self.state.set_token_balance(tx.order1_account_id, tx.token_id_1to2, balance1);
        /*

        self.balance_trees
            .get_mut(&tx.order1_account_id)
            .unwrap()
            .set_value(tx.token_id_1to2, balance1);
            */
        // account2 after sending, before receiving
        let mut balance2 = account2_balance_sell;
        balance2.sub_assign(&tx.amount_2to1);

        self.state.set_token_balance(tx.order2_account_id, tx.token_id_2to1, balance2);
        /*
        self.balance_trees
            .get_mut(&tx.order2_account_id)
            .unwrap()
            .set_value(tx.token_id_2to1, balance2);
            */

        let mut order1_filled_sell = old_order1.filled_sell;
        order1_filled_sell.add_assign(&tx.amount_1to2);
        let mut order1_filled_buy = old_order1.filled_buy;
        order1_filled_buy.add_assign(&tx.amount_2to1);
        let new_order1 = Order {
            order_id: u32_to_fr(tx.order1_id),
            tokenbuy: u32_to_fr(tx.token_id_2to1),
            tokensell: u32_to_fr(tx.token_id_1to2),
            filled_sell: order1_filled_sell,
            filled_buy: order1_filled_buy,
            total_sell: old_order1.total_sell,
            total_buy: old_order1.total_buy,
        };
        self.state.update_order_state(tx.order1_account_id, new_order1);
        self.state.update_order_leaf(tx.order1_account_id, order1_pos, tx.order1_id);

        let mut account1_balance_buy = account1_balance_buy;
        account1_balance_buy.add_assign(&tx.amount_2to1);
        self.state
            .set_token_balance(tx.order1_account_id, tx.token_id_2to1, account1_balance_buy);

        let mut order2_filled_sell = old_order2.filled_sell;
        order2_filled_sell.add_assign(&tx.amount_2to1);
        let mut order2_filled_buy = old_order2.filled_buy;
        order2_filled_buy.add_assign(&tx.amount_1to2);
        let new_order2 = Order {
            order_id: u32_to_fr(tx.order2_id),
            tokenbuy: u32_to_fr(tx.token_id_1to2),
            tokensell: u32_to_fr(tx.token_id_2to1),
            filled_sell: order2_filled_sell,
            filled_buy: order2_filled_buy,
            total_sell: old_order2.total_sell,
            total_buy: old_order2.total_buy,
        };
        self.state.update_order_state(tx.order2_account_id, new_order2);
        self.state.update_order_leaf(tx.order2_account_id, order2_pos, tx.order2_id);

        let mut account2_balance_buy = account2_balance_buy;
        account2_balance_buy.add_assign(&tx.amount_1to2);
        self.state
            .set_token_balance(tx.order2_account_id, tx.token_id_1to2, account2_balance_buy);

        raw_tx.balance_path3 = self.state.balance_proof(tx.order1_account_id, tx.token_id_2to1).path_elements;
        raw_tx.balance_path1 = self.state.balance_proof(tx.order2_account_id, tx.token_id_1to2).path_elements;
        raw_tx.account_path1 = self.state.account_proof(tx.order2_account_id).path_elements;
        raw_tx.order_root1 = self.state.get_account(tx.order2_account_id).order_root;

        encoded_tx[tx_detail_idx::NEW_ORDER1_ID] = new_order1.order_id;
        encoded_tx[tx_detail_idx::NEW_ORDER1_TOKEN_SELL] = new_order1.tokensell;
        encoded_tx[tx_detail_idx::NEW_ORDER1_FILLED_SELL] = new_order1.filled_sell;
        encoded_tx[tx_detail_idx::NEW_ORDER1_AMOUNT_SELL] = new_order1.total_sell;
        encoded_tx[tx_detail_idx::NEW_ORDER1_TOKEN_BUY] = new_order1.tokenbuy;
        encoded_tx[tx_detail_idx::NEW_ORDER1_FILLED_BUY] = new_order1.filled_buy;
        encoded_tx[tx_detail_idx::NEW_ORDER1_AMOUNT_BUY] = new_order1.total_buy;

        encoded_tx[tx_detail_idx::NEW_ORDER2_ID] = new_order2.order_id;
        encoded_tx[tx_detail_idx::NEW_ORDER2_TOKEN_SELL] = new_order2.tokensell;
        encoded_tx[tx_detail_idx::NEW_ORDER2_FILLED_SELL] = new_order2.filled_sell;
        encoded_tx[tx_detail_idx::NEW_ORDER2_AMOUNT_SELL] = new_order2.total_sell;
        encoded_tx[tx_detail_idx::NEW_ORDER2_TOKEN_BUY] = new_order2.tokenbuy;
        encoded_tx[tx_detail_idx::NEW_ORDER2_FILLED_BUY] = new_order2.filled_buy;
        encoded_tx[tx_detail_idx::NEW_ORDER2_AMOUNT_BUY] = new_order2.total_buy;

        encoded_tx[tx_detail_idx::TOKEN_ID1] = new_order1.tokensell;
        encoded_tx[tx_detail_idx::TOKEN_ID2] = new_order2.tokenbuy;

        raw_tx.payload = encoded_tx.to_vec();
        raw_tx.root_after = self.state.root();
        self.add_raw_tx(raw_tx);
    }

    pub fn nop(&mut self) {
        // assume we already have initialized the account tree and the balance tree
        let trivial_proof = self.state.trivial_state_proof();
        let encoded_tx = [Fr::zero(); TX_LENGTH];
        let raw_tx = RawTx {
            tx_type: TxType::Nop,
            payload: encoded_tx.to_vec(),
            balance_path0: trivial_proof.balance_path.clone(),
            balance_path1: trivial_proof.balance_path.clone(),
            balance_path2: trivial_proof.balance_path.clone(),
            balance_path3: trivial_proof.balance_path,
            order_path0: self.state.trivial_order_path_elements(),
            order_path1: self.state.trivial_order_path_elements(),
            order_root0: trivial_proof.order_root,
            order_root1: trivial_proof.order_root,
            account_path0: trivial_proof.account_path.clone(),
            account_path1: trivial_proof.account_path,
            root_before: self.state.root(),
            root_after: self.state.root(),
        };
        self.add_raw_tx(raw_tx);
    }

    pub fn flush_with_nop(&mut self) {
        while self.buffered_txs.len() % self.n_tx != 0 {
            self.nop();
        }
    }
}