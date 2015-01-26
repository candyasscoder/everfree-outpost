use std::collections::HashMap;
use std::iter::repeat;
use serialize::json::Json;

use physics::Shape;
use physics::v3::V3;
use types::{BlockId, TemplateId};


#[derive(Show)]
pub struct ParseError(pub String);


pub struct Data {
    pub block_data: BlockData,
    pub object_templates: ObjectTemplates,
}

impl Data {
    pub fn from_json(block_json: Json,
                     template_json: Json) -> Result<Data, ParseError> {
        let block_data = try!(BlockData::from_json(block_json));
        let object_templates = try!(ObjectTemplates::from_json(template_json,
                                                               &block_data));
        Ok(Data {
            block_data: block_data,
            object_templates: object_templates,
        })
    }
}


pub struct BlockData {
    shapes: Vec<Shape>,
    name_to_id: HashMap<String, BlockId>,
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


impl BlockData {
    pub fn from_json(json: Json) -> Result<BlockData, ParseError> {
        let blocks = get_convert!(json, "blocks", as_array,
                                  "at top level");

        let mut shapes = repeat(Shape::Empty).take(blocks.len()).collect::<Vec<_>>();
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
            name_to_id.insert(String::from_str(name), i as BlockId);
        }

        Ok(BlockData {
            shapes: shapes,
            name_to_id: name_to_id,
        })
    }

    pub fn shape(&self, id: BlockId) -> Shape {
        self.shapes.as_slice().get(id as usize).map(|&x| x).unwrap_or(Shape::Empty)
    }

    pub fn get_id(&self, name: &str) -> BlockId {
        self.find_id(name).unwrap_or_else(|| panic!("unknown block id: {}", name))
    }

    pub fn find_id(&self, name: &str) -> Option<BlockId> {
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
                name: String::from_str(name),
            });
            name_to_id.insert(String::from_str(name), i as TemplateId);
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
        self.templates.as_slice().get(id as usize)
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
