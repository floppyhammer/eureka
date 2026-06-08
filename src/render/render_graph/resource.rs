use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

/// 资源类型枚举
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResourceType {
    Texture,
    Buffer,
    BindGroup,
    Pipeline,
    Sampler,
}

/// 类型化资源 ID
#[derive(Debug, Clone)]
pub struct ResourceId<T> {
    name: String,
    _marker: PhantomData<T>,
}

impl<T> ResourceId<T> {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            _marker: PhantomData,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl<T> PartialEq for ResourceId<T> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl<T> Eq for ResourceId<T> {}

impl<T> Hash for ResourceId<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl<T> Display for ResourceId<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// 纹理资源键（包含纹理的完整规格）
#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub struct TextureKey {
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
    pub usage: wgpu::TextureUsages,
}

/// 缓冲区资源键
#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub struct BufferKey {
    pub size: u64,
    pub usage: wgpu::BufferUsages,
}

/// 资源规格（用于创建资源时的参数）
/// 目前主要用于纹理资源的声明
#[derive(Debug, Clone)]
pub enum ResourceSpec {
    Texture(TextureKey),
    Buffer(BufferKey),
    /// 占位符，用于声明需要 BindGroup 但不指定具体布局
    BindGroup,
    /// 占位符，用于声明需要 Pipeline 但不指定具体布局
    Pipeline,
    /// 占位符，用于声明需要 Sampler
    Sampler,
}

/// 资源声明（节点使用）
#[derive(Debug, Clone)]
pub struct ResourceDecl {
    pub id: ResourceId<()>,
    pub spec: ResourceSpec,
    pub optional: bool,
}

/// 节点资源声明集合
pub struct NodeResources {
    pub inputs: Vec<ResourceDecl>,
    pub outputs: Vec<ResourceDecl>,
}

impl NodeResources {
    pub fn new() -> Self {
        Self {
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }

    pub fn input(mut self, id: ResourceId<()>, spec: ResourceSpec) -> Self {
        self.inputs.push(ResourceDecl {
            id,
            spec,
            optional: false,
        });
        self
    }

    pub fn optional_input(mut self, id: ResourceId<()>, spec: ResourceSpec) -> Self {
        self.inputs.push(ResourceDecl {
            id,
            spec,
            optional: true,
        });
        self
    }

    pub fn output(mut self, id: ResourceId<()>, spec: ResourceSpec) -> Self {
        self.outputs.push(ResourceDecl {
            id,
            spec,
            optional: false,
        });
        self
    }
}

/// 预定义的标准资源 ID（使用函数而非静态常量）
pub mod standard_resources {
    use super::ResourceId;

    // 颜色缓冲区
    pub fn main_color() -> ResourceId<()> {
        ResourceId::new("main_color")
    }

    pub fn fxaa_color() -> ResourceId<()> {
        ResourceId::new("fxaa_color")
    }

    // 深度缓冲区
    pub fn main_depth() -> ResourceId<()> {
        ResourceId::new("main_depth")
    }

    // SSAO 相关
    pub fn ssao_normal() -> ResourceId<()> {
        ResourceId::new("ssao_normal")
    }

    pub fn ssao_output() -> ResourceId<()> {
        ResourceId::new("ssao_output")
    }

    // 阴影相关
    pub fn shadow_map() -> ResourceId<()> {
        ResourceId::new("shadow_map")
    }

    // 最终输出
    pub fn final_output() -> ResourceId<()> {
        ResourceId::new("final_output")
    }
}