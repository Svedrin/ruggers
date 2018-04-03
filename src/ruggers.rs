use std::collections::HashMap;
use std::rc::Rc;

const GEN_MASK: u64 = 0xFFFFFFFFFFFFFFF0;

#[derive(Debug, Eq, PartialEq)]
pub struct RuggedRecord {
    pub birth_gen: u64,
    pub key:       String,
    pub value:     String,
}

impl RuggedRecord {
    pub fn new(birth_gen: u64, key: String, value: String) -> Rc<RuggedRecord> {
        Rc::new(RuggedRecord {
            birth_gen:  birth_gen,
            key:        key,
            value:      value,
        })
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct RuggedGeneration {
    this_gen:   u64,
    data:       HashMap<String,Rc<RuggedRecord>>
}

impl RuggedGeneration {
    /**
     * Create an empty RuggedGeneration without predecessors.
     */
    pub fn new_root(node_id: u8) -> RuggedGeneration {
        RuggedGeneration {
            this_gen:   node_id as u64,
            data: HashMap::new(),
        }
    }

    /**
     * Create a new child of ours.
     */
    fn new_child(&self) -> RuggedGeneration {
        RuggedGeneration {
            this_gen:   self.this_gen + 16,
            data:       self.data.clone(),
        }
    }

    pub fn this_gen(&self) -> u64 {
        self.this_gen
    }

    /**
     * Get a value wrapped in a RuggedValue.
     */
    pub fn get(&self, key: &String) -> Option<Rc<RuggedRecord>> {
        if let Some(val) = self.data.get(key) {
            Some(val.clone())
        } else {
            None
        }
    }

    /**
     * Store a string in our hashmap, creating a new RuggedGeneration.
     */
    pub fn store(&self, key: &String, value: &String) -> RuggedGeneration {
        let mut next_gen = self.new_child();
        let val = RuggedRecord::new(
            next_gen.this_gen(),
            key.to_owned(),
             value.to_owned()
        );
        next_gen.data.insert(key.to_owned(), val);
        next_gen
    }

    /**
     * Merge a Record we received from a cluster peer into our data set.
     */
    pub fn merge(&self, record: Rc<RuggedRecord>) -> Option<RuggedGeneration> {
        // Find out if we *can* merge.
        // We can, unless there's a collision.
        // A collision means that
        // a) the key exists here too, AND
        // b) it was born at the same time or later as the record we're getting now.
        if let Some(ref my_record) = self.data.get(&record.as_ref().key) {
            if my_record.birth_gen >= record.as_ref().birth_gen {
                return None;
            }
        }
        let mut next_gen = self.new_child();
        assert!(
            // Make sure the generation sent by our peer is not in the future.
            // For instance if next_gen is 0x51, it's ok to accept 0x52 values
            // into it because the gen AFTER that is going to be 0x61, aka > 0x52.
            // Accepting 0x62 however doesn't work because the following gen
            // would still be smaller than that.
            // This situation indicates we lost sync somehow, so let's crash
            // and burn in that case.
            (record.as_ref().birth_gen & GEN_MASK) <= (next_gen.this_gen & GEN_MASK),
            format!("Record Gen {:x} is too far in the future (we're {:x})",
                    record.as_ref().birth_gen & GEN_MASK, next_gen.this_gen & GEN_MASK)
        );
        next_gen.data.insert(record.as_ref().key.to_owned(), record);
        Some(next_gen)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /**
     * Test the simple case: Repeatedly storing stuff.
     * Each time we call `store`, a new generation should be created.
     * In the end we should see all data, each with the correct birth_gen.
     */
    #[test]
    fn test_store() {
        let gen = RuggedGeneration::new_root(1)
            .store(&String::from("Hallo1"), &String::from("omfg1"))
            .store(&String::from("Hallo2"), &String::from("omfg2"))
            .store(&String::from("Hallo3"), &String::from("omfg3"))
            .store(&String::from("Hallo4"), &String::from("omfg4"));
        assert_eq!(
            gen.get(&String::from("Hallo1")).unwrap(),
            RuggedRecord::new(
                1 + (16 * 1), String::from("Hallo1"), String::from("omfg1")
            )
        );
        assert_eq!(
            gen.get(&String::from("Hallo2")).unwrap(),
            RuggedRecord::new(
                1 + (16 * 2), String::from("Hallo2"), String::from("omfg2")
            )
        );
        assert_eq!(
            gen.get(&String::from("Hallo3")).unwrap(),
            RuggedRecord::new(
                1 + (16 * 3), String::from("Hallo3"), String::from("omfg3")
            )
        );
        assert_eq!(
            gen.get(&String::from("Hallo4")).unwrap(),
            RuggedRecord::new(
                1 + (16 * 4), String::from("Hallo4"), String::from("omfg4")
            )
        );
        assert_eq!(gen.this_gen(), 1 + (16 * 4));
        assert!(
            gen.get(&String::from("Hallo4")).unwrap().as_ref().birth_gen <= gen.this_gen()
        );
    }

    /**
     * Test a merge where the keys we receive don't exist locally.
     */
    #[test]
    fn test_merge_success_new_key() {
        let rec3 = RuggedRecord::new(
            2 + (16 * 2),
            String::from("Hallo3"),
            String::from("yolo3")
        );
        let rec4 = RuggedRecord::new(
            2 + (16 * 3),
            String::from("Hallo4"),
            String::from("yolo4")
        );
        let gen = RuggedGeneration::new_root(1)
            .store(&String::from("Hallo1"), &String::from("omfg1"))
            .store(&String::from("Hallo2"), &String::from("omfg2"))
            .merge(rec3).unwrap()
            .merge(rec4).unwrap();
        assert_eq!(
            gen.get(&String::from("Hallo1")).unwrap(),
            RuggedRecord::new(
                1 + (16 * 1), String::from("Hallo1"), String::from("omfg1")
            )
        );
        assert_eq!(
            gen.get(&String::from("Hallo2")).unwrap(),
            RuggedRecord::new(
                1 + (16 * 2), String::from("Hallo2"), String::from("omfg2")
            )
        );
        assert_eq!(
            gen.get(&String::from("Hallo3")).unwrap(),
            RuggedRecord::new(
                2 + (16 * 2), String::from("Hallo3"), String::from("yolo3")
            )
        );
        assert_eq!(
            gen.get(&String::from("Hallo4")).unwrap(),
            RuggedRecord::new(
                2 + (16 * 3), String::from("Hallo4"), String::from("yolo4")
            )
        );
        assert_eq!(gen.this_gen(), 1 + (16 * 4));
        assert!(
            gen.get(&String::from("Hallo4")).unwrap().as_ref().birth_gen <= gen.this_gen()
        );
    }

    /**
     * Test a merge where the keys do exist locally, but do not cause a collision.
     * (Remote's Records are newer than our own.)
     */
    #[test]
    fn test_merge_success_existing_key() {
        let rec3 = RuggedRecord::new(
            2 + (16 * 3),
            String::from("Hallo3"),
            String::from("yolo3")
        );
        let rec4 = RuggedRecord::new(
            2 + (16 * 4),
            String::from("Hallo4"),
            String::from("yolo4")
        );
        let gen = RuggedGeneration::new_root(1)
            .store(&String::from("Hallo1"), &String::from("omfg1"))
            .store(&String::from("Hallo2"), &String::from("omfg2"))
            .store(&String::from("Hallo3"), &String::from("omfg3"))
            .store(&String::from("Hallo4"), &String::from("omfg4"))
            .merge(rec3).unwrap()
            .merge(rec4).unwrap();
        assert_eq!(
            gen.get(&String::from("Hallo1")).unwrap(),
            RuggedRecord::new(
                1 + (16 * 1), String::from("Hallo1"), String::from("omfg1")
            )
        );
        assert_eq!(
            gen.get(&String::from("Hallo2")).unwrap(),
            RuggedRecord::new(
                1 + (16 * 2), String::from("Hallo2"), String::from("omfg2")
            )
        );
        assert_eq!(
            gen.get(&String::from("Hallo3")).unwrap(),
            RuggedRecord::new(
                2 + (16 * 3), String::from("Hallo3"), String::from("yolo3")
            )
        );
        assert_eq!(
            gen.get(&String::from("Hallo4")).unwrap(),
            RuggedRecord::new(
                2 + (16 * 4), String::from("Hallo4"), String::from("yolo4")
            )
        );
        assert_eq!(gen.this_gen(), 1 + (16 * 6));
        assert!(
            gen.get(&String::from("Hallo4")).unwrap().as_ref().birth_gen <= gen.this_gen()
        );
    }

    /**
     * Test a merge where the keys exist locally and do cause a collision.
     * (Remote's Records are older than our own.)
     */
    #[test]
    fn test_merge_failed_existing_key() {
        let rec3 = RuggedRecord::new(
            2 + (16 * 1),
            String::from("Hallo3"),
            String::from("yolo3")
        );
        let rec4 = RuggedRecord::new(
            2 + (16 * 3),
            String::from("Hallo4"),
            String::from("yolo4")
        );
        let gen = RuggedGeneration::new_root(1)
            .store(&String::from("Hallo1"), &String::from("omfg1"))
            .store(&String::from("Hallo2"), &String::from("omfg2"))
            .store(&String::from("Hallo3"), &String::from("omfg3"))
            .store(&String::from("Hallo4"), &String::from("omfg4"));
        assert_eq!(gen.merge(rec3), None);
        assert_eq!(gen.merge(rec4), None);
        assert_eq!(
            gen.get(&String::from("Hallo1")).unwrap(),
            RuggedRecord::new(
                1 + (16 * 1), String::from("Hallo1"), String::from("omfg1")
            )
        );
        assert_eq!(
            gen.get(&String::from("Hallo2")).unwrap(),
            RuggedRecord::new(
                1 + (16 * 2), String::from("Hallo2"), String::from("omfg2")
            )
        );
        assert_eq!(
            gen.get(&String::from("Hallo3")).unwrap(),
            RuggedRecord::new(
                1 + (16 * 3), String::from("Hallo3"), String::from("omfg3")
            )
        );
        assert_eq!(
            gen.get(&String::from("Hallo4")).unwrap(),
            RuggedRecord::new(
                1 + (16 * 4), String::from("Hallo4"), String::from("omfg4")
            )
        );
        assert_eq!(gen.this_gen(), 1 + (16 * 4));
        assert!(
            gen.get(&String::from("Hallo4")).unwrap().as_ref().birth_gen <= gen.this_gen()
        );
    }

    /**
     * Test a merge where the keys do exist locally, but do not cause a collision.
     * (Remote's Records are newer than our own.)
     * In this case we do not craft the peer's records by hand, but use another
     * RuggedGeneration tree instead.
     */
    #[test]
    fn test_merge_two_gen_trees() {
        let gen_b = RuggedGeneration::new_root(2)
            .store(&String::from("Hallo1"), &String::from("omfg1"))
            .store(&String::from("Hallo2"), &String::from("omfg2"))
            .store(&String::from("Hallo3"), &String::from("omfg3"))
            .store(&String::from("Hallo4"), &String::from("omfg4"))
            .store(&String::from("Hallo3"), &String::from("yolo3"))
            .store(&String::from("Hallo4"), &String::from("yolo4"));
        let gen_a = RuggedGeneration::new_root(1)
            .store(&String::from("Hallo1"), &String::from("omfg1"))
            .store(&String::from("Hallo2"), &String::from("omfg2"))
            .store(&String::from("Hallo3"), &String::from("omfg3"))
            .store(&String::from("Hallo4"), &String::from("omfg4"))
            .merge(gen_b.get(&String::from("Hallo3")).unwrap()).unwrap()
            .merge(gen_b.get(&String::from("Hallo4")).unwrap()).unwrap()
            .store(&String::from("Hallo5"), &String::from("omfg5"));
        assert_eq!(
            gen_a.get(&String::from("Hallo1")).unwrap(),
            RuggedRecord::new(
                1 + (16 * 1), String::from("Hallo1"), String::from("omfg1")
            )
        );
        assert_eq!(
            gen_a.get(&String::from("Hallo2")).unwrap(),
            RuggedRecord::new(
                1 + (16 * 2), String::from("Hallo2"), String::from("omfg2")
            )
        );
        assert_eq!(
            gen_a.get(&String::from("Hallo3")).unwrap(),
            RuggedRecord::new(
                2 + (16 * 5), String::from("Hallo3"), String::from("yolo3")
            )
        );
        assert_eq!(
            gen_a.get(&String::from("Hallo4")).unwrap(),
            RuggedRecord::new(
                2 + (16 * 6), String::from("Hallo4"), String::from("yolo4")
            )
        );
        assert_eq!(
            gen_a.get(&String::from("Hallo5")).unwrap(),
            RuggedRecord::new(
                1 + (16 * 7), String::from("Hallo5"), String::from("omfg5")
            )
        );
        assert_eq!(gen_a.this_gen(), 1 + (16 * 7));
        assert!(
            gen_a.get(&String::from("Hallo4")).unwrap().as_ref().birth_gen <= gen_a.this_gen()
        );
    }
}
