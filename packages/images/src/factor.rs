// ! 设置图片默认值

#[derive(Clone, Debug)]
pub struct Factor {
    pub quality: f32,    // 品质: 0 - 100
    pub size_ratio: f32, // 压缩比例: 0 - 1
}

impl Factor {
    #[allow(dead_code)]
    pub fn new(quality: f32, size_ratio: f32) -> Self {
        if (quality > 0. && quality <= 100.) && (size_ratio > 0. && size_ratio <= 1.) {
            Self { quality, size_ratio }
        } else {
            panic!("Wrong Factor argument!");
        }
    }

    pub fn quality(&self) -> f32 {
        return self.quality;
    }

    pub fn size_ratio(&self) -> f32 {
        return self.size_ratio;
    }
}

impl Default for Factor {
    fn default() -> Self {
        Self { quality: 80., size_ratio: 0.8 }
    }
}
