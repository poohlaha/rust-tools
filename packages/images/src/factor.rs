// ! 设置图片默认值

#[derive(Clone, Debug)]
pub struct Factor {
    pub quality: f32,    // 品质: 0 - 100
    pub size_ratio: f32, // 压缩比例: 0 - 1
}

impl Factor {

    pub fn quality(&self) -> f32 {
        return self.quality;
    }

    pub fn size_ratio(&self) -> f32 {
        return self.size_ratio;
    }

    pub fn get_default_quality(&self) -> f32 {
        return 80.0
    }

    pub fn get_default_size_ratio(&self) -> f32 {
        return 0.8
    }
}

impl Default for Factor {
    fn default() -> Self {
        Self { quality: 80., size_ratio: 0.8 }
    }
}
