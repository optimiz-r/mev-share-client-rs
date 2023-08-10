use mev_share_rs::Builder;
use std::{collections::HashSet, sync::LazyLock};

pub static BUILDERS: LazyLock<HashSet<Builder>> = LazyLock::new(|| {
    let mut builders = HashSet::new();
    builders.insert(Builder::Flashbots);
    builders.insert(Builder::Rsync);
    builders.insert(Builder::BeaverBuild);
    builders.insert(Builder::Builder0x69);
    builders.insert(Builder::Titan);
    builders.insert(Builder::EigenPhi);
    builders.insert(Builder::BobaBuilder);
    builders
});
