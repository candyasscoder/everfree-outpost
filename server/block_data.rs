use std::collections::HashMap;
use serialize::json::Json;

use physics;
use physics::Shape;
use types::BlockId;


#[deriving(Show)]
pub struct ParseError(pub String);

pub struct BlockData {
    shapes: Vec<Shape>,
    name_to_id: HashMap<String, BlockId>,
}

macro_rules! unwrap {
    ($e:expr, $str:expr $($extra:tt)*) => {
        match $e {
            Some(x) => x,
            None => return Err(ParseError(format!($str $($extra)*))),
        }
    }
}

impl BlockData {
    pub fn from_json(json: Json) -> Result<BlockData, ParseError> {
        let blocks = unwrap!(json.find("blocks").and_then(|j| j.as_array()),
                             "missing array \"blocks\" at top level");

        let mut shapes = Vec::from_elem(blocks.len(), Shape::Empty);
        let mut name_to_id = HashMap::new();

        for (i, block) in blocks.iter().enumerate() {
            let name = unwrap!(block.find("name").and_then(|j| j.as_string()),
                               "missing string \"name\" for block {}", i);
            let shape_str = unwrap!(block.find("shape").and_then(|j| j.as_string()),
                                    "missing string \"shape\" for block {} ({})", i, name);

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
            name_to_id.insert(name.into_string(), i as BlockId);
        }

        Ok(BlockData {
            shapes: shapes,
            name_to_id: name_to_id,
        })
    }

    pub fn shape(&self, id: BlockId) -> Shape {
        self.shapes.as_slice().get(id as uint).map(|&x| x).unwrap_or(Shape::Empty)
    }

    pub fn get_id(&self, name: &str) -> BlockId {
        self.find_id(name).unwrap_or_else(|| panic!("unknown block id: {}", name))
    }

    pub fn find_id(&self, name: &str) -> Option<BlockId> {
        self.name_to_id.get(name).map(|&x| x)
    }
}
