// https://github1s.com/Fluidex/circuits/blob/HEAD/helper.ts/binary_merkle_tree.ts

use fnv::FnvHashMap;
use franklin_crypto::bellman::bn256::{Bn256, Fr};
use franklin_crypto::rescue::bn256::Bn256RescueParams;
use franklin_crypto::rescue::rescue_hash;
use lazy_static::lazy_static;
use std::iter;

type LeafIndex = u32;
type LeafType = Fr;
type ValueMap = FnvHashMap<LeafIndex, LeafType>;

lazy_static! {
    pub static ref RESCUE_PARAMS: Bn256RescueParams = Bn256RescueParams::new_checked_2_into_1();
}

fn hash(inputs: &[Fr]) -> Fr {
    rescue_hash::<Bn256>(&RESCUE_PARAMS, &inputs)[0]
}

pub struct MerkleProofN<const LENGTH: usize> {
    pub root: LeafType,
    pub leaf: LeafType,
    pub path_elements: Vec<[LeafType; LENGTH]>,
}
type MerkleProof = MerkleProofN<1>;

// TODO: use leaf_index/leaf_type as generics
pub struct Tree {
    pub height: usize,
    // precalculate mid hashes, so we don't have to store the empty nodes
    default_nodes: Vec<LeafType>,

    // In `data`, we only store the nodes with non empty values
    // data[0] is leaf nodes, and data[-1] is root
    // the `logical size` of data[0] is of size 2**height
    data: Vec<ValueMap>,
}

impl Tree {
    pub fn new(height: usize, default_leaf_node_value: LeafType) -> Self {
        // check overflow
        let _ = 2u32.checked_pow(height as u32).expect("tree depth error, overflow");
        // 2**height leaves, and the total height of the tree is
        //self.height = height;
        let mut default_nodes = vec![default_leaf_node_value];
        for i in 0..height {
            default_nodes.push(hash(&[default_nodes[i], default_nodes[i]]));
        }
        let data = iter::repeat_with(ValueMap::default).take(height + 1).collect();
        Self {
            height,
            default_nodes,
            data,
        }
    }
    pub fn max_leaf_num(&self) -> u32 {
        2u32.checked_pow(self.height as u32).unwrap()
    }
    /*
    pub fn print(dense = true, empty_label = 'None') {
      console.log(`Tree(height: ${self.height}, leaf_num: ${Math.pow(2, self.height)}, non_empty_leaf_num: ${self.data[0].size})`);
      if (dense) {
        for (let i = 0; i < self.data.length; i++) {
          process.stdout.write(i == 0 ? 'Leaves\t' : `Mid(${i})\t`);
          for (let j = 0; j < Math.pow(2, self.height - i); j++) {
            process.stdout.write(self.data[i].has(big_int(j)) ? self.data[i].get(big_int(j)).to_string() : empty_label);
            process.stdout.write(',');
          }
          process.stdout.write('\n');
        }
      } else {
        for (let i = 0; i < self.data.length; i++) {
          process.stdout.write(i == 0 ? 'Leaves\t' : `Mid(${i})\t`);
          for (let [k, v] of self.data[i].entries()) {
            process.stdout.write(`${k}:${v},`);
          }
          process.stdout.write('\n');
        }
      }
    }
    */
    pub fn sibling_idx(&self, n: LeafIndex) -> LeafIndex {
        if n % 2 == 1 {
            n - 1
        } else {
            n + 1
        }
    }
    pub fn parent_idx(&self, n: LeafIndex) -> LeafIndex {
        n >> 1
    }
    pub fn get_value(&self, level: usize, idx: u32) -> LeafType {
        *self.data[level].get(&idx).unwrap_or(&self.default_nodes[level])
    }
    pub fn get_leaf(&self, idx: u32) -> LeafType {
        self.get_value(0, idx)
    }
    fn recalculate_parent(&mut self, level: usize, idx: u32) {
        let lhs = self.get_value(level - 1, idx * 2);
        let rhs = self.get_value(level - 1, idx * 2 + 1);
        let new_hash = hash(&[lhs, rhs]);
        self.data[level].insert(idx, new_hash);
    }
    pub fn set_value(&mut self, idx: u32, value: LeafType) {
        let mut idx = idx;
        if self.get_leaf(idx) == value {
            return;
        }
        if idx >= self.max_leaf_num() {
            panic!("invalid tree idx {}", idx);
        }
        self.data[0].insert(idx, value);
        for i in 1..=self.height {
            idx = self.parent_idx(idx);
            self.recalculate_parent(i, idx);
        }
    }
    // of course there is no such thing 'parallel' in Js
    // self function is only used as pseudo code for future Rust rewrite
    pub fn set_value_parallel(&mut self, idx: u32, value: LeafType) {
        #[derive(Default)]
        struct HashCacheItem {
            inputs: Vec<LeafType>,
            result: LeafType,
        }
        // the precalculating can be done parallelly
        let mut precalculated = Vec::<HashCacheItem>::default();
        let mut cur_idx = idx;
        let mut cur_value = value;
        for i in 0..self.height {
            let pair = if cur_idx % 2 == 0 {
                [cur_value, self.get_value(i, cur_idx + 1)]
            } else {
                [self.get_value(i, cur_idx - 1), cur_value]
            };
            cur_value = hash(&pair);
            cur_idx = self.parent_idx(cur_idx);
            precalculated.push(HashCacheItem {
                inputs: pair.to_vec(),
                result: cur_value,
            });
        }
        // apply the precalculated
        let mut cache_miss = false;
        cur_idx = idx;
        //cur_value = value;
        self.data[0].insert(idx, value);
        for i in 0..self.height {
            let pair = if cur_idx % 2 == 0 {
                [self.get_value(i, cur_idx), self.get_value(i, cur_idx + 1)]
            } else {
                [self.get_value(i, cur_idx - 1), self.get_value(i, cur_idx)]
            };
            cur_idx = self.parent_idx(cur_idx);
            if !cache_miss {
                // TODO: is the `cache_miss` shortcut really needed? comparing bigint is quite cheap compared to hash
                // `cache_miss` makes codes more difficult to read
                if !(precalculated[i].inputs[0] == pair[0] || precalculated[i].inputs[1] == pair[1]) {
                    // Due to self is a merkle tree, future caches will all be missed.
                    // precalculated becomes totally useless now
                    cache_miss = true;
                    precalculated.clear();
                }
            }
            if cache_miss {
                self.data[i + 1].insert(cur_idx, hash(&pair));
            } else {
                self.data[i + 1].insert(cur_idx, precalculated[i].result);
            }
        }
    }
    pub fn fill_with_leaves_vec(&mut self, leaves: &[LeafType]) {
        if leaves.len() != self.max_leaf_num() as usize {
            panic!("invalid leaves size {}", leaves.len());
        }
        // TODO: optimize here
        for (i, item) in leaves.iter().enumerate() {
            self.set_value(i as u32, *item);
        }
    }
    pub fn fill_with_leaves_map(&mut self, leaves: std::collections::HashMap<LeafIndex, LeafType>) {
        for (k, v) in leaves.iter() {
            self.set_value(*k, *v);
        }
    }
    pub fn get_root(&self) -> LeafType {
        self.get_value(self.data.len() - 1, 0)
    }
    pub fn get_proof(&self, index: u32) -> MerkleProof {
        let mut index = index;
        let leaf = self.get_leaf(index);
        let mut path_elements = Vec::new();
        for i in 0..self.height {
            path_elements.push([self.get_value(i, self.sibling_idx(index))]);
            index = self.parent_idx(index);
        }
        MerkleProof {
            root: self.get_root(),
            path_elements,
            leaf,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use franklin_crypto::bellman::{Field, PrimeField};
    use std::time::Instant;
    //use test::Bencher;
    #[test]
    #[ignore]
    fn bench_tree() {
        let h = 20;
        let mut tree = Tree::new(h, Fr::zero());
        for i in 0..100 {
            let start = Instant::now();
            let inner_count = 100;
            for j in 0..inner_count {
                if j % 100 == 0 {
                    println!("progress {} {}", i, j);
                }
                tree.set_value(j, Fr::from_str(&format!("{}", j + i)).unwrap());
            }
            // 2021.03.15(Apple M1): typescript: 100 ops takes 4934ms
            // 2021.03.26(Apple M1): rust:       100 ops takes 1160ms
            println!("{} ops takes {}ms", inner_count, start.elapsed().as_millis());
        }
    }
}