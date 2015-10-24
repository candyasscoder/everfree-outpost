use rand::Rng;

use libserver_types::*;

pub type TableIndex = u16;
pub type Weight = u16;
pub type Chance = u8;


pub enum ItemTable {
    Item(ItemId, u8, u8),
    Choose(Vec<(TableIndex, Weight)>, i32),
    Multi(Vec<(TableIndex, Chance)>),
}

impl ItemTable {
    pub fn eval<R: Rng>(&self, tables: &[ItemTable], rng: &mut R, output: &mut Vec<(ItemId, u8)>) {
        use self::ItemTable::*;
        match *self {
            Item(item_id, min, max) => {
                let amount = rng.gen_range(min as u16, max as u16 + 1) as u8;
                output.push((item_id, amount));
            },
            Choose(ref variants, weight_sum) => {
                let mut x = rng.gen_range(0, weight_sum);
                for &(table_idx, weight) in variants {
                    x -= weight as i32;
                    if x < 0 {
                        tables[table_idx as usize].eval(tables, rng, output);
                        break;
                    }
                }
            },
            Multi(ref parts) => {
                for &(table_idx, chance) in parts {
                    if chance < 100 && rng.gen_range(0, 100) >= chance {
                        continue;
                    }
                    tables[table_idx as usize].eval(tables, rng, output);
                }
            },
        }
    }
}


pub enum StructureTable {
    Structure(TemplateId),
    Choose(Vec<(TableIndex, Weight)>, i32),
}

impl StructureTable {
    pub fn eval<R: Rng>(&self,
                        tables: &[StructureTable],
                        rng: &mut R,
                        output: &mut Option<TemplateId>) {
        use self::StructureTable::*;
        match *self {
            Structure(structure_id) => {
                *output = Some(structure_id);
            },
            Choose(ref variants, weight_sum) => {
                let mut x = rng.gen_range(0, weight_sum);
                for &(table_idx, weight) in variants {
                    x -= weight as i32;
                    if x < 0 {
                        tables[table_idx as usize].eval(tables, rng, output);
                        break;
                    }
                }
            },
        }
    }
}

