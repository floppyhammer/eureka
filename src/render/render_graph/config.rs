/// 抗锯齿模式选择
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AntiAliasingMode {
    None,
    Fxaa,
    Taa,
    TaaAndFxaa,
}

/// 阴影渲染配置
#[derive(Debug, Clone, PartialEq)]
pub struct ShadowConfig {
    pub enabled: bool,
    pub map_resolution: u32,
    pub cascade_count: u32,
}

impl Default for ShadowConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            map_resolution: 2048,
            cascade_count: 4,
        }
    }
}

/// 屏幕空间环境光遮蔽 (SSAO) 配置
#[derive(Debug, Clone, PartialEq)]
pub struct SsaoConfig {
    pub enabled: bool,
    pub sample_count: u32,
    pub radius: f32,
}

impl Default for SsaoConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sample_count: 64,
            radius: 0.5,
        }
    }
}

/// 屏幕空间全局光照 (SSGI) 配置
#[derive(Debug, Clone, PartialEq)]
pub struct SsgiConfig {
    pub enabled: bool,
    pub ray_steps: u32,
}

impl Default for SsgiConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ray_steps: 32,
        }
    }
}

/// 屏幕空间反射 (SSR) 配置
#[derive(Debug, Clone, PartialEq)]
pub struct SsrConfig {
    pub enabled: bool,
    pub max_steps: u32,
}

impl Default for SsrConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_steps: 64,
        }
    }
}

/// 体积光/散射配置
#[derive(Debug, Clone, PartialEq)]
pub struct VolumetricConfig {
    pub enabled: bool,
    pub sample_count: u32,
}

impl Default for VolumetricConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sample_count: 32,
        }
    }
}

/// Bloom 泛光配置
#[derive(Debug, Clone, PartialEq)]
pub struct BloomConfig {
    pub enabled: bool,
    pub intensity: f32,
    pub threshold: f32,
}

impl Default for BloomConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            intensity: 1.0,
            threshold: 1.0,
        }
    }
}

/// ToneMapping 色彩映射配置
#[derive(Debug, Clone, PartialEq)]
pub struct ToneMappingConfig {
    pub enabled: bool,
}

impl Default for ToneMappingConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// 渲染图全局及子系统配置结构体
#[derive(Debug, Clone, PartialEq)]
pub struct RenderGraphConfig {
    pub shadow: ShadowConfig,
    pub ssao: SsaoConfig,
    pub ssgi: SsgiConfig,
    pub ssr: SsrConfig,
    pub volumetric: VolumetricConfig,
    pub bloom: BloomConfig,
    pub aa_mode: AntiAliasingMode,
    pub tonemapping: ToneMappingConfig,
}

impl Default for RenderGraphConfig {
    fn default() -> Self {
        Self::medium()
    }
}

impl RenderGraphConfig {
    /// 低画质预设 (Low): 关闭高消耗特效，保留基础画质与 FXAA
    pub fn low() -> Self {
        Self {
            shadow: ShadowConfig {
                enabled: true,
                map_resolution: 1024,
                cascade_count: 2,
            },
            ssao: SsaoConfig {
                enabled: false,
                sample_count: 16,
                radius: 0.5,
            },
            ssgi: SsgiConfig {
                enabled: false,
                ray_steps: 16,
            },
            ssr: SsrConfig {
                enabled: false,
                max_steps: 32,
            },
            volumetric: VolumetricConfig {
                enabled: false,
                sample_count: 16,
            },
            bloom: BloomConfig {
                enabled: true,
                intensity: 0.8,
                threshold: 1.0,
            },
            aa_mode: AntiAliasingMode::Fxaa,
            tonemapping: ToneMappingConfig { enabled: true },
        }
    }

    /// 中画质预设 (Medium): 开启基础 SSAO 与 TAA
    pub fn medium() -> Self {
        Self {
            shadow: ShadowConfig {
                enabled: true,
                map_resolution: 2048,
                cascade_count: 4,
            },
            ssao: SsaoConfig {
                enabled: true,
                sample_count: 32,
                radius: 0.5,
            },
            ssgi: SsgiConfig {
                enabled: false,
                ray_steps: 32,
            },
            ssr: SsrConfig {
                enabled: false,
                max_steps: 64,
            },
            volumetric: VolumetricConfig {
                enabled: false,
                sample_count: 32,
            },
            bloom: BloomConfig {
                enabled: true,
                intensity: 1.0,
                threshold: 1.0,
            },
            aa_mode: AntiAliasingMode::Taa,
            tonemapping: ToneMappingConfig { enabled: true },
        }
    }

    /// 高画质预设 (High): 开启 SSAO, TAA, SSR, 体积光 (默认)
    pub fn high() -> Self {
        Self {
            shadow: ShadowConfig {
                enabled: true,
                map_resolution: 2048,
                cascade_count: 4,
            },
            ssao: SsaoConfig {
                enabled: true,
                sample_count: 64,
                radius: 0.5,
            },
            ssgi: SsgiConfig {
                enabled: false,
                ray_steps: 32,
            },
            ssr: SsrConfig {
                enabled: true,
                max_steps: 64,
            },
            volumetric: VolumetricConfig {
                enabled: true,
                sample_count: 32,
            },
            bloom: BloomConfig {
                enabled: true,
                intensity: 1.0,
                threshold: 1.0,
            },
            aa_mode: AntiAliasingMode::Taa,
            tonemapping: ToneMappingConfig { enabled: true },
        }
    }

    /// 极致画质预设 (Ultra): 全特效开启（包含 SSGI 与更高采样率）
    pub fn ultra() -> Self {
        Self {
            shadow: ShadowConfig {
                enabled: true,
                map_resolution: 4096,
                cascade_count: 4,
            },
            ssao: SsaoConfig {
                enabled: true,
                sample_count: 64,
                radius: 0.5,
            },
            ssgi: SsgiConfig {
                enabled: true,
                ray_steps: 64,
            },
            ssr: SsrConfig {
                enabled: true,
                max_steps: 128,
            },
            volumetric: VolumetricConfig {
                enabled: true,
                sample_count: 64,
            },
            bloom: BloomConfig {
                enabled: true,
                intensity: 1.0,
                threshold: 1.0,
            },
            aa_mode: AntiAliasingMode::TaaAndFxaa,
            tonemapping: ToneMappingConfig { enabled: true },
        }
    }
}
