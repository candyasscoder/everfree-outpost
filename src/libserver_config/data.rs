use std::borrow::ToOwned;
use std::collections::HashMap;
use std::iter::repeat;
use rand::Rng;
use rustc_serialize::json::Json;

use libserver_types::*;

use loot::{TableIndex, Weight, Chance, ItemTable, StructureTable};


#[derive(Debug)]
pub struct ParseError(pub String);


pub struct Data {
    pub block_data: BlockData,
    pub item_data: ItemData,
    pub recipes: RecipeData,
    pub structure_templates: StructureTemplates,
    pub animations: AnimationData,
    pub loot_tables: LootTables,
}

impl Data {
    pub fn from_json(block_json: Json,
                     item_json: Json,
                     recipe_json: Json,
                     structure_template_json: Json,
                     animation_json: Json,
                     loot_table_json: Json) -> Result<Data, ParseError> {
        let block_data = try!(BlockData::from_json(block_json));
        let item_data = try!(ItemData::from_json(item_json));
        let recipes = try!(RecipeData::from_json(recipe_json));
        let structure_templates = try!(StructureTemplates::from_json(structure_template_json));
        let animations = try!(AnimationData::from_json(animation_json));
        let loot_tables = try!(LootTables::from_json(loot_table_json));
        Ok(Data {
            block_data: block_data,
            item_data: item_data,
            recipes: recipes,
            structure_templates: structure_templates,
            animations: animations,
            loot_tables: loot_tables,
        })
    }
}



macro_rules! fail {
    ($msg:expr) => {
        fail!($msg,)
    };
    ($msg:expr, $($extra:tt)*) => {
        Err(ParseError(format!($msg, $($extra)*)))
    };
}

macro_rules! expect {
    ($e:expr, $str:expr) => {
        expect!($e, $str,)
    };
    ($e:expr, $str:expr, $($extra:tt)*) => {
        match $e {
            Some(x) => x,
            None => return Err(ParseError(format!($str, $($extra)*))),
        }
    };
}

macro_rules! find_convert {
    ($json:expr, $key:expr, $convert:ident, $where_:expr) => {
        find_convert!($json, $key, $convert, $where_,)
    };
    ($json:expr, $key:expr, $convert:ident, $where_:expr, $($extra:tt)*) => {{
        let key = $key;
        match $json.find(key) {
            Some(j) => match j.$convert() {
                Some(x) => Ok(x),
                None => fail!(concat!("failed to convert key \"{}\" with {} ", $where_),
                              key, stringify!($convert), $($extra)*),
            },
            None => fail!(concat!("missing key \"{}\" ", $where_),
                          key, $($extra)*),
        }
    }};
}

macro_rules! get_convert {
    ($json:expr, $key:expr, $convert:ident, $where_:expr) => {
        get_convert!($json, $key, $convert, $where_,)
    };
    ($json:expr, $key:expr, $convert:ident, $where_:expr, $($extra:tt)*) => {
        try!(find_convert!($json, $key, $convert, $where_, $($extra)*))
    };
}

macro_rules! convert {
    ($json:expr, $convert:ident, $what:expr) => {
        convert!($expr, $convert, $what,)
    };
    ($json:expr, $convert:ident, $what:expr, $($extra:tt)*) => {{
        match json.$convert() {
            Some(x) => Ok(x),
            None => fail!(concat!("failed to convert ", $what, " with {}"),
                          $($extra)*, stringify!($convert)),
        },
    }};
}


pub struct BlockData {
    shapes: Vec<Shape>,
    names: Vec<String>,
    name_to_id: HashMap<String, BlockId>,
}

impl BlockData {
    pub fn from_json(json: Json) -> Result<BlockData, ParseError> {
        let blocks = expect!(json.as_array(),
                                "found non-array at top level");

        let mut shapes = repeat(Shape::Empty).take(blocks.len()).collect::<Vec<_>>();
        let mut names = Vec::with_capacity(shapes.len());
        let mut name_to_id = HashMap::new();

        for (i, block) in blocks.iter().enumerate() {
            let name = get_convert!(block, "name", as_string,
                                    "for block {}", i);
            let shape_str = get_convert!(block, "shape", as_string,
                                         "for block {} ({})", i, name);

            let shape = match shape_str {
                "empty" => Shape::Empty,
                "floor" => Shape::Floor,
                "solid" => Shape::Solid,
                "ramp_n" => Shape::RampN,
                _ => return fail!("invalid shape \"{}\" for block {} ({})",
                                  shape_str, i, name),
            };
            shapes[i] = shape;
            names.push(name.to_owned());
            name_to_id.insert(name.to_owned(), i as BlockId);
        }

        Ok(BlockData {
            shapes: shapes,
            names: names,
            name_to_id: name_to_id,
        })
    }

    pub fn shape(&self, id: BlockId) -> Shape {
        self.shapes.get(id as usize).map(|&x| x).unwrap_or(Shape::Empty)
    }

    pub fn name(&self, id: BlockId) -> &str {
        &*self.names[id as usize]
    }

    pub fn get_id(&self, name: &str) -> BlockId {
        self.find_id(name).unwrap_or_else(|| panic!("unknown block id: {}", name))
    }

    pub fn find_id(&self, name: &str) -> Option<BlockId> {
        self.name_to_id.get(name).map(|&x| x)
    }
}


pub struct ItemData {
    names: Vec<String>,
    name_to_id: HashMap<String, ItemId>,
}

impl ItemData {
    pub fn from_json(json: Json) -> Result<ItemData, ParseError> {
        let items = expect!(json.as_array(),
                                "found non-array at top level");

        let mut names = Vec::with_capacity(items.len());
        let mut name_to_id = HashMap::new();

        for (i, item) in items.iter().enumerate() {
            let name = get_convert!(item, "name", as_string,
                                    "for item {}", i);

            names.push(name.to_owned());
            name_to_id.insert(name.to_owned(), i as ItemId);
        }

        Ok(ItemData {
            names: names,
            name_to_id: name_to_id,
        })
    }

    pub fn name(&self, id: ItemId) -> &str {
        &*self.names[id as usize]
    }

    pub fn get_name(&self, id: ItemId) -> Option<&str> {
        self.names.get(id as usize).map(|s| &**s)
    }

    pub fn get_id(&self, name: &str) -> ItemId {
        self.find_id(name).unwrap_or_else(|| panic!("unknown item id: {}", name))
    }

    pub fn find_id(&self, name: &str) -> Option<ItemId> {
        self.name_to_id.get(name).map(|&x| x)
    }
}


pub struct Recipe {
    pub name: String,
    pub inputs: HashMap<ItemId, u8>,
    pub outputs: HashMap<ItemId, u8>,
    pub station: Option<TemplateId>,
}

pub struct RecipeData {
    recipes: Vec<Recipe>,
    name_to_id: HashMap<String, RecipeId>,
}

impl RecipeData {
    pub fn from_json(json: Json) -> Result<RecipeData, ParseError> {
        let recipes_json = expect!(json.as_array(),
                                "found non-array at top level");

        let mut recipes = Vec::with_capacity(recipes_json.len());
        let mut name_to_id = HashMap::new();

        for (i, recipe) in recipes_json.iter().enumerate() {
            let name = get_convert!(recipe, "name", as_string,
                                    "for recipe {}", i);
            let station = match find_convert!(recipe, "station", as_i64,
                                              "for recipe {}", i) {
                Ok(station) => Some(station as TemplateId),
                Err(_) => None,
            };

            fn build_map(list: &[Json], what: &str, i: usize) -> Result<HashMap<ItemId, u8>, ParseError> {
                let mut map = HashMap::new();
                for (j, entry) in list.iter().enumerate() {
                    let entry = expect!(entry.as_array(),
                                        "failed to convert recipe {} {} {}", i, what, j);
                    if entry.len() != 2 {
                        return fail!("bad length for recipe {} {} {}", i, what, j);
                    }
                    let item = expect!(entry[0].as_i64(),
                                       "failed to convert recipe {} {} {} item", i, what, j);
                    let count = expect!(entry[1].as_i64(),
                                        "failed to convert recipe {} {} {} count", i, what, j);
                    map.insert(item as ItemId, count as u8);
                }
                Ok(map)
            }

            let inputs = get_convert!(recipe, "inputs", as_array,
                                      "for recipe {}", i);
            let inputs = try!(build_map(&**inputs, "input", i));

            let outputs = get_convert!(recipe, "outputs", as_array,
                                       "for recipe {}", i);
            let outputs = try!(build_map(&**outputs, "input", i));

            recipes.push(Recipe {
                name: name.to_owned(),
                inputs: inputs,
                outputs: outputs,
                station: station,
            });
            name_to_id.insert(name.to_owned(), i as RecipeId); 
        }

        Ok(RecipeData {
            recipes: recipes,
            name_to_id: name_to_id,
        })
    }

    pub fn recipe(&self, id: RecipeId) -> &Recipe {
        &self.recipes[id as usize]
    }

    pub fn get_recipe(&self, id: RecipeId) -> Option<&Recipe> {
        self.recipes.get(id as usize)
    }

    pub fn get_id(&self, name: &str) -> RecipeId {
        self.find_id(name).unwrap_or_else(|| panic!("unknown recipe id: {}", name))
    }

    pub fn find_id(&self, name: &str) -> Option<RecipeId> {
        self.name_to_id.get(name).map(|&x| x)
    }
}


pub struct StructureTemplate {
    pub name: String,
    pub size: V3,
    pub shape: Vec<Shape>,
    pub layer: u8,
}

pub struct StructureTemplates {
    templates: Vec<StructureTemplate>,
    name_to_id: HashMap<String, TemplateId>,
}

impl StructureTemplates {
    pub fn from_json(json: Json) -> Result<StructureTemplates, ParseError> {
        let templates = expect!(json.as_array(),
                                "found non-array at top level");

        let mut by_id = Vec::with_capacity(templates.len());
        let mut name_to_id = HashMap::new();

        for (i, template) in templates.iter().enumerate() {
            let name = get_convert!(template, "name", as_string,
                                    "for template {}", i);
            let size_arr = get_convert!(template, "size", as_array,
                                        "for template {} ({})", i, name);
            let shape_arr = get_convert!(template, "shape", as_array,
                                         "for template {} ({})", i, name);
            let layer = get_convert!(template, "layer", as_i64,
                                     "for template {} ({})", i, name);

            if size_arr.len() != 3 {
                return fail!("wrong number of elements in templates[{}].size ({})",
                             i, name);
            }

            let size_x = expect!(size_arr[0].as_i64(),
                                 "non-integer in templates[{}].size ({})", i, name);
            let size_y = expect!(size_arr[1].as_i64(),
                                 "non-integer in templates[{}].size ({})", i, name);
            let size_z = expect!(size_arr[2].as_i64(),
                                 "non-integer in templates[{}].size ({})", i, name);

            let size = V3::new(size_x as i32,
                               size_y as i32,
                               size_z as i32);

            let mut shape = Vec::with_capacity(shape_arr.len());
            for (j, shape_json) in shape_arr.iter().enumerate() {
                let shape_disr = expect!(shape_json.as_i64(),
                                         "non-integer at templates[{}].shape[{}] ({})",
                                         i, j, name);
                let shape_enum = expect!(Shape::from_primitive(shape_disr as usize),
                                         "invalid shape {} at templates[{}].shape[{}] ({})",
                                         shape_disr, i, j, name);
                shape.push(shape_enum);
            }

            info!("parsed template: {}", name);
            by_id.push(StructureTemplate {
                name: name.to_owned(),
                size: size,
                shape: shape,
                layer: layer as u8,
            });
            name_to_id.insert(name.to_owned(), i as TemplateId);
        }

        Ok(StructureTemplates {
            templates: by_id,
            name_to_id: name_to_id,
        })
    }

    pub fn template(&self, id: TemplateId) -> &StructureTemplate {
        self.get_template(id).unwrap()
    }

    pub fn get_template(&self, id: TemplateId) -> Option<&StructureTemplate> {
        self.templates.get(id as usize)
    }

    pub fn get_id(&self, name: &str) -> TemplateId {
        self.find_id(name).unwrap_or_else(|| panic!("unknown structure template id: {}", name))
    }

    pub fn find_id(&self, name: &str) -> Option<TemplateId> {
        self.name_to_id.get(name).map(|&x| x)
    }

    pub fn get_by_id(&self, name: &str) -> &StructureTemplate {
        self.template(self.get_id(name))
    }
}


pub struct Animation {
    pub name: String,
    pub framerate: u32,
    pub length: u32,
}

pub struct AnimationData {
    animations: Vec<Animation>,
    name_to_id: HashMap<String, AnimId>,
}

impl AnimationData {
    pub fn from_json(json: Json) -> Result<AnimationData, ParseError> {
        let animations = expect!(json.as_array(),
                                "found non-array at top level");

        let mut by_id = Vec::with_capacity(animations.len());
        let mut name_to_id = HashMap::new();

        for (i, animation) in animations.iter().enumerate() {
            let name = get_convert!(animation, "name", as_string,
                                    "for animation {}", i);
            let framerate = get_convert!(animation, "framerate", as_i64,
                                        "for animation {} ({})", i, name);
            let length = get_convert!(animation, "length", as_i64,
                                      "for animation {} ({})", i, name);

            by_id.push(Animation {
                name: name.to_owned(),
                framerate: framerate as u32,
                length: length as u32,
            });
            name_to_id.insert(name.to_owned(), i as AnimId);
        }

        Ok(AnimationData {
            animations: by_id,
            name_to_id: name_to_id,
        })
    }

    pub fn animation(&self, id: AnimId) -> &Animation {
        self.get_animation(id).unwrap()
    }

    pub fn get_animation(&self, id: AnimId) -> Option<&Animation> {
        self.animations.get(id as usize)
    }

    pub fn get_id(&self, name: &str) -> AnimId {
        self.find_id(name).unwrap_or_else(|| panic!("unknown animation id: {}", name))
    }

    pub fn find_id(&self, name: &str) -> Option<AnimId> {
        self.name_to_id.get(name).map(|&x| x)
    }

    pub fn get_by_id(&self, name: &str) -> &Animation {
        self.animation(self.get_id(name))
    }
}


pub struct LootTables {
    pub item: Box<[ItemTable]>,
    pub item_by_name: HashMap<String, TableIndex>,
    pub structure: Box<[StructureTable]>,
    pub structure_by_name: HashMap<String, TableIndex>,
}

impl LootTables {
    pub fn from_json(json: Json) -> Result<LootTables, ParseError> {
        let items = get_convert!(json, "items", as_array,
                                 "in top-level object");
        let mut item_tables = Vec::with_capacity(items.len());
        let mut item_by_name = HashMap::new();
        for (i, table) in items.iter().enumerate() {
            let ty = get_convert!(table, "type", as_string,
                                  "for item table {}", i);

            let t = 
                match ty {
                    "object" => {
                        let item_id = get_convert!(table, "id", as_i64,
                                                   "for item table {}", i);
                        let min_count = get_convert!(table, "min_count", as_i64,
                                                     "for item table {}", i);
                        let max_count = get_convert!(table, "max_count", as_i64,
                                                     "for item table {}", i);
                        ItemTable::Item(item_id as ItemId, min_count as u8, max_count as u8)
                    },
                    "choose" => {
                        let variants_json = get_convert!(table, "variants", as_array,
                                                         "for item table {}", i);
                        let mut variants = Vec::with_capacity(variants_json.len());
                        let mut weight_sum = 0;
                        for (j, v) in variants_json.iter().enumerate() {
                            let id = get_convert!(v, "id", as_i64,
                                                  "for variant {} of item table {}", j, i);
                            let weight = get_convert!(v, "weight", as_i64,
                                                      "for variant {} of item table {}", j, i);
                            variants.push((id as TableIndex, weight as Weight));
                            weight_sum += weight as i32;
                        }
                        ItemTable::Choose(variants, weight_sum)
                    },
                    "multi" => {
                        let parts_json = get_convert!(table, "parts", as_array,
                                                      "for item table {}", i);
                        let mut parts = Vec::with_capacity(parts_json.len());
                        for (j, v) in parts_json.iter().enumerate() {
                            let id = get_convert!(v, "id", as_i64,
                                                  "for part {} of item table {}", j, i);
                            let chance = get_convert!(v, "chance", as_i64,
                                                      "for part {} of item table {}", j, i);
                            parts.push((id as TableIndex, chance as Chance));
                        }
                        ItemTable::Multi(parts)
                    },
                    _ => return fail!("bad type \"{}\" for item table {}", ty, i),
                };
            item_tables.push(t);

            match table.find("name").and_then(|n| n.as_string()) {
                Some(name) => { item_by_name.insert(name.to_owned(), i as TableIndex); },
                None => {},
            }
        }


        let structures = get_convert!(json, "structures", as_array,
                                      "in top-level object");
        let mut structure_tables = Vec::with_capacity(structures.len());
        let mut structure_by_name = HashMap::new();
        for (i, table) in structures.iter().enumerate() {
            let ty = get_convert!(table, "type", as_string,
                                  "for structure table {}", i);

            let t = 
                match ty {
                    "object" => {
                        let structure_id = get_convert!(table, "id", as_i64,
                                                        "for structure table {}", i);
                        StructureTable::Structure(structure_id as TemplateId)
                    },
                    "choose" => {
                        let variants_json = get_convert!(table, "variants", as_array,
                                                         "for structure table {}", i);
                        let mut variants = Vec::with_capacity(variants_json.len());
                        let mut weight_sum = 0;
                        for (j, v) in variants_json.iter().enumerate() {
                            let id = get_convert!(v, "id", as_i64,
                                                  "for variant {} of structure table {}", j, i);
                            let weight = get_convert!(v, "weight", as_i64,
                                                      "for variant {} of structure table {}", j, i);
                            variants.push((id as TableIndex, weight as Weight));
                            weight_sum += weight as i32;
                        }
                        StructureTable::Choose(variants, weight_sum)
                    },
                    _ => return fail!("bad type \"{}\" for structure table {}", ty, i),
                };
            structure_tables.push(t);

            match table.find("name").and_then(|n| n.as_string()) {
                Some(name) => { structure_by_name.insert(name.to_owned(), i as TableIndex); },
                None => {},
            }
        }


        Ok(LootTables {
            item: item_tables.into_boxed_slice(),
            item_by_name: item_by_name,
            structure: structure_tables.into_boxed_slice(),
            structure_by_name: structure_by_name,
        })
    }

    pub fn eval_item_table<R: Rng>(&self, rng: &mut R, name: &str) -> Vec<(ItemId, u8)> {
        let mut result = Vec::new();
        let id = self.item_by_name[name];
        self.item[id as usize].eval(&self.item, rng, &mut result);
        result
    }

    pub fn eval_structure_table<R: Rng>(&self, rng: &mut R, name: &str) -> Option<TemplateId> {
        let mut result = None;
        let id = self.structure_by_name[name];
        self.structure[id as usize].eval(&self.structure, rng, &mut result);
        result
    }
}
