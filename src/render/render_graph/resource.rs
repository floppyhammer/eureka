use crate::render::Texture;
use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

/// 资源类型标签
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureTag;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferTag;

/// 类型化资源 ID
#[derive(Debug)]
pub struct ResourceId<T> {
    name: String,
    _marker: PhantomData<T>,
}

impl<T> Clone for ResourceId<T> {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            _marker: PhantomData,
        }
    }
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

    /// 擦除类型信息，转为通用 ID
    pub fn erase(self) -> ResourceId<()> {
        ResourceId {
            name: self.name,
            _marker: PhantomData,
        }
    }
}

pub type TextureId = ResourceId<TextureTag>;
pub type BufferId = ResourceId<BufferTag>;

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
    pub format: Option<wgpu::TextureFormat>,
    pub usage: wgpu::TextureUsages,
    pub layers: u32,
}

impl Default for TextureKey {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            format: Some(wgpu::TextureFormat::Rgba8UnormSrgb),
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            layers: 1,
        }
    }
}

/// 缓冲区资源键
#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub struct BufferKey {
    pub size: u64,
    pub usage: wgpu::BufferUsages,
}

/// 采样器资源键（包含采样器的完整规格）
#[derive(PartialEq, Clone, Copy, Debug)]
pub struct SamplerKey {
    pub address_mode_u: wgpu::AddressMode,
    pub address_mode_v: wgpu::AddressMode,
    pub address_mode_w: wgpu::AddressMode,
    pub mag_filter: wgpu::FilterMode,
    pub min_filter: wgpu::FilterMode,
    pub mipmap_filter: wgpu::FilterMode,
    pub compare: Option<wgpu::CompareFunction>,
    pub lod_min_clamp: f32,
    pub lod_max_clamp: f32,
}

impl Eq for SamplerKey {}

impl std::hash::Hash for SamplerKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.address_mode_u.hash(state);
        self.address_mode_v.hash(state);
        self.address_mode_w.hash(state);
        self.mag_filter.hash(state);
        self.min_filter.hash(state);
        self.mipmap_filter.hash(state);
        self.compare.hash(state);
        // f32 的 hash 需要特殊处理
        self.lod_min_clamp.to_bits().hash(state);
        self.lod_max_clamp.to_bits().hash(state);
    }
}

impl Default for SamplerKey {
    fn default() -> Self {
        Self {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: None,
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
        }
    }
}

/// 包含克隆后的句柄，不绑定生命周期
pub struct ResolvedTransientTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub id: u64,
    pub view_id: u64,
    pub(crate) handle: Texture,
}

impl ResolvedTransientTexture {
    /// 获取一个稳定的视图及其 ID，用于 BindGroup 缓存优化
    pub fn get_view(&self, desc: &wgpu::TextureViewDescriptor) -> (wgpu::TextureView, u64) {
        self.handle.get_view(desc)
    }
}

/// 池化缓冲区包装，包含物理 Buffer 和其唯一 ID
#[derive(Clone)]
pub struct PooledBuffer {
    pub buffer: wgpu::Buffer,
    pub id: u64,
}

/// BindGroup 缓存键
#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub struct BindGroupKey {
    pub layout_name: String,
    pub resource_ids: Vec<u64>,
}

/// 资源池化键的统一包装
#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub enum ResourceKey {
    Texture(TextureKey),
    Buffer(BufferKey),
}

/// 运行时物理资源的统一包装
#[derive(Clone)]
pub enum VirtualResource {
    Texture(Texture),
    Buffer(PooledBuffer),
}

impl VirtualResource {
    pub fn id(&self) -> u64 {
        match self {
            VirtualResource::Texture(t) => t.id,
            VirtualResource::Buffer(b) => b.id,
        }
    }
}

/// 资源规格（用于创建资源时的参数）
#[derive(Debug, Clone)]
pub enum ResourceSpec {
    Texture(TextureKey),
    Buffer(BufferKey),
    Sampler(SamplerKey),
}

impl ResourceSpec {
    /// 快速创建一个通用的纹理规格
    pub fn texture(
        width: u32,
        height: u32,
        format: Option<wgpu::TextureFormat>,
        usage: wgpu::TextureUsages,
        layers: u32,
    ) -> Self {
        Self::Texture(TextureKey {
            width,
            height,
            format,
            usage,
            layers,
        })
    }

    /// 快速创建一个通用的缓冲区规格
    pub fn buffer(size: u64, usage: wgpu::BufferUsages) -> Self {
        Self::Buffer(BufferKey { size, usage })
    }

    pub fn merge(&mut self, other: &Self) {
        match (self, other) {
            (ResourceSpec::Texture(a), ResourceSpec::Texture(b)) => {
                a.usage |= b.usage;
                a.width = a.width.max(b.width);
                a.height = a.height.max(b.height);
                a.layers = a.layers.max(b.layers);
            }
            (ResourceSpec::Buffer(a), ResourceSpec::Buffer(b)) => {
                a.usage |= b.usage;
                a.size = a.size.max(b.size);
            }
            _ => {}
        }
    }
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
    pub internals: Vec<ResourceDecl>,
}

impl NodeResources {
    pub fn new() -> Self {
        Self {
            inputs: Vec::new(),
            outputs: Vec::new(),
            internals: Vec::new(),
        }
    }

    pub fn input<T>(mut self, id: ResourceId<T>, spec: ResourceSpec) -> Self {
        self.inputs.push(ResourceDecl {
            id: id.erase(),
            spec,
            optional: false,
        });
        self
    }

    pub fn optional_input<T>(mut self, id: ResourceId<T>, spec: ResourceSpec) -> Self {
        self.inputs.push(ResourceDecl {
            id: id.erase(),
            spec,
            optional: true,
        });
        self
    }

    pub fn output<T>(mut self, id: ResourceId<T>, spec: ResourceSpec) -> Self {
        self.outputs.push(ResourceDecl {
            id: id.erase(),
            spec,
            optional: false,
        });
        self
    }

    pub fn internal<T>(mut self, id: ResourceId<T>, spec: ResourceSpec) -> Self {
        self.internals.push(ResourceDecl {
            id: id.erase(),
            spec,
            optional: false,
        });
        self
    }
}

/// 预定义的标准资源 ID（使用函数而非静态常量）
pub mod standard_resources {
    use super::{BufferId, ResourceId, TextureId};

    // 颜色缓冲区（HDR）
    pub fn main_color() -> TextureId {
        ResourceId::new("main_color")
    }

    // 深度缓冲区
    pub fn main_depth() -> TextureId {
        ResourceId::new("main_depth")
    }

    // SDR
    pub fn hdr_resolved() -> TextureId {
        ResourceId::new("hdr_resolved")
    }

    // 最终输出，一般是 Surface
    pub fn final_output() -> TextureId {
        ResourceId::new("final_output")
    }

    pub fn camera_buffer() -> BufferId {
        ResourceId::new("camera_buffer")
    }

    pub fn material_storage_buffer() -> BufferId {
        ResourceId::new("material_storage_buffer")
    }

    // Sprite node
    pub fn sprite_vertex_buffer() -> BufferId {
        ResourceId::new("sprite_vertex_buffer")
    }

    pub fn sprite_index_buffer() -> BufferId {
        ResourceId::new("sprite_index_buffer")
    }

    pub fn global_instance_buffer() -> BufferId {
        ResourceId::new("global_instance_buffer")
    }

    pub fn mesh_metadata_buffer() -> BufferId {
        ResourceId::new("mesh_metadata_buffer")
    }

    pub fn cull_visible_instance_buffer() -> BufferId {
        ResourceId::new("cull_visible_instance_buffer")
    }

    pub fn cull_indirect_buffer() -> BufferId {
        ResourceId::new("cull_indirect_buffer")
    }

    pub fn cull_params_uniform() -> BufferId {
        ResourceId::new("cull_params_uniform")
    }

    pub fn shadow_cascade_buffer() -> BufferId {
        ResourceId::new("shadow_cascade_buffer")
    }

    pub fn directional_shadow_camera_buffer() -> BufferId {
        ResourceId::new("directional_shadow_camera_buffer")
    }

    pub fn point_shadow_camera_buffer() -> BufferId {
        ResourceId::new("point_shadow_camera_buffer")
    }

    pub fn light_uniform_buffer() -> BufferId {
        ResourceId::new("light_uniform_buffer")
    }

    pub fn fxaa_settings() -> BufferId {
        ResourceId::new("fxaa_settings")
    }

    // SSAO 相关
    pub fn ssao_depth() -> TextureId {
        ResourceId::new("ssao_depth")
    }

    pub fn ssao_normal() -> TextureId {
        ResourceId::new("ssao_normal")
    }

    pub fn ssao_output() -> TextureId {
        ResourceId::new("ssao_output")
    }

    pub fn ssao_blur() -> TextureId {
        ResourceId::new("ssao_blur")
    }

    // 阴影相关
    pub fn directional_shadow_map() -> TextureId {
        ResourceId::new("directional_shadow_map")
    }

    pub fn point_shadow_map() -> TextureId {
        ResourceId::new("point_shadow_map")
    }
}
