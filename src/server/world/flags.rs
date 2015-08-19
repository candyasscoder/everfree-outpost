bitflags! {
    flags TerrainChunkFlags: u32 {
        /// Terrain generation for this chunk is still running in a background thread.
        const TC_GENERATION_PENDING = 0x00000001,
    }
}

bitflags! {
    flags StructureFlags: u32 {
        const S_HAS_SAVE_HOOKS      = 0x00000001,
    }
}
