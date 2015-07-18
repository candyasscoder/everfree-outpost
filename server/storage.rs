use std::borrow::Cow;
use std::fmt::Debug;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use types::*;


const DATA_DIR: &'static str = "data";
const BLOCK_DATA_FILE: &'static str = "blocks.json";
const ITEM_DATA_FILE: &'static str = "items.json";
const RECIPE_DATA_FILE: &'static str = "recipes.json";
const OLD_TEMPLATE_DATA_FILE: &'static str = "objects.json";
const TEMPLATE_DATA_FILE: &'static str = "structures.json";
const ANIMATION_DATA_FILE: &'static str = "animations.json";

const SCRIPT_DIR: &'static str = "scripts";

const SAVE_DIR: &'static str = "save";
const CLIENT_DIR: &'static str = "clients";
const PLANE_DIR: &'static str = "planes";
const TERRAIN_CHUNK_DIR: &'static str = "terrain_chunks";
const WORLD_FILE_NAME: &'static str = "world.dat";
const MISC_FILE_NAME: &'static str = "misc.dat";
const AUTH_DB_FILE_NAME: &'static str = "auth.sqlite";


pub struct Storage {
    base: PathBuf,
}

impl Storage {
    pub fn new<P: AsRef<Path>>(base: &P) -> Storage {
        let base = base.as_ref().to_owned();
        fs::create_dir_all(base.join(SAVE_DIR).join(CLIENT_DIR)).unwrap();
        fs::create_dir_all(base.join(SAVE_DIR).join(PLANE_DIR)).unwrap();
        fs::create_dir_all(base.join(SAVE_DIR).join(TERRAIN_CHUNK_DIR)).unwrap();

        Storage {
            base: base,
        }
    }

    pub fn data_path(&self, file: &str) -> PathBuf {
        self.base.join(DATA_DIR).join(file)
    }

    pub fn open_block_data(&self) -> File {
        File::open(self.data_path(BLOCK_DATA_FILE)).unwrap()
    }

    pub fn open_item_data(&self) -> File {
        File::open(self.data_path(ITEM_DATA_FILE)).unwrap()
    }

    pub fn open_recipe_data(&self) -> File {
        File::open(self.data_path(RECIPE_DATA_FILE)).unwrap()
    }

    pub fn open_old_template_data(&self) -> File {
        File::open(self.data_path(OLD_TEMPLATE_DATA_FILE)).unwrap()
    }

    pub fn open_template_data(&self) -> File {
        File::open(self.data_path(TEMPLATE_DATA_FILE)).unwrap()
    }

    pub fn open_animation_data(&self) -> File {
        File::open(self.data_path(ANIMATION_DATA_FILE)).unwrap()
    }

    pub fn script_dir(&self) -> PathBuf {
        self.base.join(SCRIPT_DIR)
    }

    pub fn world_path(&self) -> PathBuf {
        self.base.join(SAVE_DIR).join(WORLD_FILE_NAME)
    }

    pub fn misc_path(&self) -> PathBuf {
        self.base.join(SAVE_DIR).join(MISC_FILE_NAME)
    }

    pub fn auth_db_path(&self) -> PathBuf {
        self.base.join(SAVE_DIR).join(AUTH_DB_FILE_NAME)
    }

    pub fn client_path(&self, name: &str) -> PathBuf {
        self.base.join(SAVE_DIR).join(CLIENT_DIR)
            .join(&*sanitize(name))
            .with_extension("client")
    }

    pub fn plane_path(&self, stable_pid: Stable<PlaneId>) -> PathBuf {
        self.base.join(SAVE_DIR).join(PLANE_DIR)
            .join(format!("{:x}", stable_pid.unwrap()))
            .with_extension("plane")
    }

    pub fn terrain_chunk_path(&self, stable_tcid: Stable<TerrainChunkId>) -> PathBuf {
        self.base.join(SAVE_DIR).join(TERRAIN_CHUNK_DIR)
            .join(format!("{:x}", stable_tcid.unwrap()))
            .with_extension("terrain_chunk")
    }

    pub fn open_world_file(&self) -> Option<File> {
        try_open_file(self.world_path())
    }

    pub fn open_misc_file(&self) -> Option<File> {
        try_open_file(self.misc_path())
    }

    pub fn open_client_file(&self, name: &str) -> Option<File> {
        try_open_file(self.client_path(name))
    }

    pub fn open_plane_file(&self, stable_pid: Stable<PlaneId>) -> Option<File> {
        try_open_file(self.plane_path(stable_pid))
    }

    pub fn open_terrain_chunk_file(&self, stable_tcid: Stable<TerrainChunkId>) -> Option<File> {
        try_open_file(self.terrain_chunk_path(stable_tcid))
    }

    pub fn create_world_file(&self) -> File {
        File::create(self.world_path()).unwrap()
    }

    pub fn create_misc_file(&self) -> File {
        File::create(self.misc_path()).unwrap()
    }

    pub fn create_client_file(&self, name: &str) -> File {
        File::create(self.client_path(name)).unwrap()
    }

    pub fn create_plane_file(&self, stable_pid: Stable<PlaneId>) -> File {
        File::create(self.plane_path(stable_pid)).unwrap()
    }

    pub fn create_terrain_chunk_file(&self, stable_tcid: Stable<TerrainChunkId>) -> File {
        File::create(self.terrain_chunk_path(stable_tcid)).unwrap()
    }
}

fn char_legal(c: char) -> bool {
    (c >= 'a' && c <= 'z') ||
    (c >= 'A' && c <= 'Z') ||
    (c >= '0' && c <= '9') ||
    (c == '_') ||
    (c == ',') ||
    (c == '.')
    // The character '-' is also legal, but we use it for encoding out-of-range characters.  '-'
    // itself gets encoded as '-x2d'.
}

fn sanitize(s: &str) -> Cow<str> {
    let mut last = 0;
    let mut buf = String::new();

    for (i, c) in s.chars().enumerate() {
        if char_legal(c) {
            continue;
        }

        buf.push_str(&s[last..i]);

        if c as u32 <= 0xff {
            buf.push_str(&*format!("-x{:02x}", c as u32));
        } else if c as u32 <= 0xffff {
            buf.push_str(&*format!("-u{:04x}", c as u32));
        } else {
            buf.push_str(&*format!("-U{:08x}", c as u32));
        }

        last = i + 1;
    }

    if last == 0 {
        Cow::Borrowed(s)
    } else {
        buf.push_str(&s[last..]);
        Cow::Owned(buf)
    }
}

fn try_open_file<P: AsRef<Path>+Debug>(path: P) -> Option<File> {
    match File::open(path) {
        Ok(f) => Some(f),
        Err(e) => {
            match e.kind() {
                io::ErrorKind::NotFound => None,
                _ => panic!("error opening file: {}", e),
            }
        },
    }
}
