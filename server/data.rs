use std::borrow::ToOwned;
use std::collections::HashMap;
use std::iter::repeat;
use rustc_serialize::json::Json;

use physics::Shape;
use physics::v3::V3;
use types::{BlockId, ItemId, TemplateId, RecipeId};


#[derive(Debug)]
pub struct ParseError(pub String);


pub struct Data {
    pub block_data: BlockData,
    pub item_data: ItemData,
    pub recipes: RecipeData,
    pub object_templates: ObjectTemplates,
    pub structure_templates: StructureTemplates,
}

impl Data {
    pub fn from_json(block_json: Json,
                     item_json: Json,
                     recipe_json: Json,
                     template_json: Json,
                     structure_template_json: Json) -> Result<Data, ParseError> {
        let block_data = try!(BlockData::from_json(block_json));
        let item_data = try!(ItemData::from_json(item_json));
        let recipes = try!(RecipeData::from_json(recipe_json));
        let object_templates = try!(ObjectTemplates::from_json(template_json,
                                                               &block_data));
        let structure_templates = try!(StructureTemplates::from_json(structure_template_json));
        Ok(Data {
            block_data: block_data,
            item_data: item_data,
            recipes: recipes,
            object_templates: object_templates,
            structure_templates: structure_templates,
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


// TODO: should allow non-contiguous IDs, since the YAML->JSON converter supports it
pub struct BlockData {
    shapes: Vec<Shape>,
    names: Vec<String>,
    name_to_id: HashMap<String, BlockId>,
}

impl BlockData {
    pub fn from_json(json: Json) -> Result<BlockData, ParseError> {
        let blocks = get_convert!(json, "blocks", as_array,
                                  "at top level");

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
                _ => {
                    let msg = format!("invalid shape \"{}\" for block {} ({})",
                                      shape_str, i, name);
                    return Err(ParseError(msg));
                },
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
        let items = get_convert!(json, "items", as_array,
                                  "at top level");

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
        let recipes_json = get_convert!(json, "recipes", as_array,
                                        "at top level");

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


pub struct ObjectTemplate {
    pub size: V3,
    pub blocks: Vec<BlockId>,
    pub name: String,
}

pub struct ObjectTemplates {
    templates: Vec<ObjectTemplate>,
    name_to_id: HashMap<String, TemplateId>,
}

impl ObjectTemplates {
    pub fn from_json(json: Json,
                     block_data: &BlockData) -> Result<ObjectTemplates, ParseError> {
        let objects = get_convert!(json, "objects", as_array,
                                     "at top level");

        let mut by_id = Vec::with_capacity(objects.len());
        let mut name_to_id = HashMap::new();

        for (i, template) in objects.iter().enumerate() {
            let name = get_convert!(template, "name", as_string,
                                    "for template {}", i);
            let size_x = get_convert!(template, "size_x", as_i64,
                                      "for template {} ({})", i, name);
            let size_y = get_convert!(template, "size_y", as_i64,
                                      "for template {} ({})", i, name);
            let size_z = get_convert!(template, "size_z", as_i64,
                                      "for template {} ({})", i, name);
            let block_strs = get_convert!(template, "blocks", as_array,
                                          "for template {} ({})", i, name);

            let size = V3::new(size_x as i32,
                               size_y as i32,
                               size_z as i32);

            let mut blocks = Vec::with_capacity(block_strs.len());
            for block_json in block_strs.iter() {
                let block_str = expect!(block_json.as_string(),
                                        "found non-string in objects[{}].blocks ({})",
                                        i, name);
                let block_id = expect!(block_data.find_id(block_str),
                                       "invalid block name \"{}\" in \
                                        objects[{}].blocks ({})",
                                       block_str, i, name);
                blocks.push(block_id);
            }

            by_id.push(ObjectTemplate {
                size: size,
                blocks: blocks,
                name: name.to_owned(),
            });
            name_to_id.insert(name.to_owned(), i as TemplateId);
        }

        Ok(ObjectTemplates {
            templates: by_id,
            name_to_id: name_to_id,
        })
    }

    pub fn template(&self, id: TemplateId) -> &ObjectTemplate {
        self.get_template(id).unwrap()
    }

    pub fn get_template(&self, id: TemplateId) -> Option<&ObjectTemplate> {
        self.templates.get(id as usize)
    }

    pub fn get_id(&self, name: &str) -> TemplateId {
        self.find_id(name).unwrap_or_else(|| panic!("unknown object template id: {}", name))
    }

    pub fn find_id(&self, name: &str) -> Option<TemplateId> {
        self.name_to_id.get(name).map(|&x| x)
    }

    pub fn get_by_id(&self, name: &str) -> &ObjectTemplate {
        self.template(self.get_id(name))
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
