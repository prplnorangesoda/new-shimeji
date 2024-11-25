pub struct ShimejiBucket {
    currently_responsible_shimejis: Vec<Shimeji>,
}

impl ShimejiBucket {
    pub fn new() -> Self {
        ShimejiBucket {
            currently_responsible_shimejis: vec![],
        }
    }
}

pub struct Shimeji;
