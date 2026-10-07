use crate::passes::ImageComp;
use spirv::{Dim, ImageFormat};

pub const TEXTURE_HANDLE_ARRAY_DESCRIPTOR_COUNT: u32 = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TextureDimension {
    D1,
    D2,
    D3,
    Cube,
    Buffer,
}

impl TextureDimension {
    pub fn to_spirv_dim(self) -> Dim {
        match self {
            TextureDimension::D1 => Dim::Dim1D,
            TextureDimension::D2 => Dim::Dim2D,
            TextureDimension::D3 => Dim::Dim3D,
            TextureDimension::Cube => Dim::DimCube,
            TextureDimension::Buffer => Dim::DimBuffer,
        }
    }

    pub fn from_spirv_dim(dim: Dim) -> Self {
        match dim {
            Dim::Dim1D => TextureDimension::D1,
            Dim::Dim3D => TextureDimension::D3,
            Dim::DimCube => TextureDimension::Cube,
            Dim::DimBuffer => TextureDimension::Buffer,
            _ => TextureDimension::D2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TextureComponent {
    Float,
    Sint,
    Uint,
}

impl TextureComponent {
    pub fn to_image_comp(self) -> ImageComp {
        match self {
            TextureComponent::Float => ImageComp::Float,
            TextureComponent::Sint => ImageComp::Sint,
            TextureComponent::Uint => ImageComp::Uint,
        }
    }

    pub fn from_image_comp(comp: ImageComp) -> Self {
        match comp {
            ImageComp::Float => TextureComponent::Float,
            ImageComp::Sint => TextureComponent::Sint,
            ImageComp::Uint => TextureComponent::Uint,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TextureFormat {
    R8,
    Rgba8,
    R16f,
    R16ui,
    Rg16f,
    Rg32f,
    R32f,
    R32i,
    R32ui,
    Rgba32i,
    Rgba32ui,
    Rgba32f,
    Rgba16f,
    Rgba8ui,
    Rgba16ui,
    Rgba8i,
    Rgba16i,
}

impl TextureFormat {
    pub fn to_spirv_format(self) -> ImageFormat {
        match self {
            TextureFormat::R8 => ImageFormat::R8,
            TextureFormat::Rgba8 => ImageFormat::Rgba8,
            TextureFormat::R16f => ImageFormat::R16f,
            TextureFormat::R16ui => ImageFormat::R16ui,
            TextureFormat::Rg16f => ImageFormat::Rg16f,
            TextureFormat::Rg32f => ImageFormat::Rg32f,
            TextureFormat::R32f => ImageFormat::R32f,
            TextureFormat::R32i => ImageFormat::R32i,
            TextureFormat::R32ui => ImageFormat::R32ui,
            TextureFormat::Rgba32i => ImageFormat::Rgba32i,
            TextureFormat::Rgba32ui => ImageFormat::Rgba32ui,
            TextureFormat::Rgba32f => ImageFormat::Rgba32f,
            TextureFormat::Rgba16f => ImageFormat::Rgba16f,
            TextureFormat::Rgba8ui => ImageFormat::Rgba8ui,
            TextureFormat::Rgba16ui => ImageFormat::Rgba16ui,
            TextureFormat::Rgba8i => ImageFormat::Rgba8i,
            TextureFormat::Rgba16i => ImageFormat::Rgba16i,
        }
    }

    pub const ALL: [Self; 17] = [
        TextureFormat::R8,
        TextureFormat::Rgba8,
        TextureFormat::R16f,
        TextureFormat::R16ui,
        TextureFormat::Rg16f,
        TextureFormat::Rg32f,
        TextureFormat::R32f,
        TextureFormat::R32i,
        TextureFormat::R32ui,
        TextureFormat::Rgba32i,
        TextureFormat::Rgba32ui,
        TextureFormat::Rgba32f,
        TextureFormat::Rgba16f,
        TextureFormat::Rgba8ui,
        TextureFormat::Rgba16ui,
        TextureFormat::Rgba8i,
        TextureFormat::Rgba16i,
    ];

    pub fn from_spirv_format(format: ImageFormat) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.to_spirv_format() == format)
    }
}

#[cfg(test)]
mod format_tests {
    use super::*;

    #[test]
    fn every_texture_format_round_trips_through_its_spirv_image_format() {
        for format in TextureFormat::ALL {
            assert_eq!(
                TextureFormat::from_spirv_format(format.to_spirv_format()),
                Some(format)
            );
        }
        assert_eq!(TextureFormat::from_spirv_format(ImageFormat::Unknown), None);
        let mapped = TextureFormat::ALL
            .into_iter()
            .map(TextureFormat::to_spirv_format)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            mapped.len(),
            TextureFormat::ALL.len(),
            "two formats share one SPIR-V ImageFormat, so from_spirv_format cannot invert them"
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TextureShape {
    pub dimension: TextureDimension,
    pub arrayed: bool,
    pub multisampled: bool,
    pub component: TextureComponent,
    pub writable: bool,
    pub array_ref: bool,
    pub array_length: Option<u32>,
    pub storage_format: Option<TextureFormat>,
}

impl TextureShape {
    pub fn descriptor_count(&self) -> u32 {
        if !self.array_ref {
            return 1;
        }
        self.array_length
            .unwrap_or(TEXTURE_HANDLE_ARRAY_DESCRIPTOR_COUNT)
    }
}

pub fn texture_shape_from_name(name: &str) -> TextureShape {
    let (writable, array_ref) = texture_access_from_name(name);
    let array_length = texture_handle_array(name).flatten();
    let shape_name = if array_ref {
        name.find("texture")
            .or_else(|| name.find("depth"))
            .and_then(|start| name.get(start..))
            .unwrap_or(name)
    } else {
        name
    };
    let head = shape_name
        .split_once('<')
        .map(|(h, _)| h)
        .unwrap_or(shape_name);
    let dimension = if head.contains("texture_buffer") {
        TextureDimension::Buffer
    } else if head.contains("1d") {
        TextureDimension::D1
    } else if head.contains("3d") {
        TextureDimension::D3
    } else if head.contains("cube") {
        TextureDimension::Cube
    } else {
        TextureDimension::D2
    };
    let arrayed = head.ends_with("_array");
    let multisampled = head.contains("_ms");
    let component = texture_component_from_name(shape_name);
    let storage_format = if writable {
        Some(storage_format_from_name(name, component))
    } else {
        None
    };
    TextureShape {
        dimension,
        arrayed,
        multisampled,
        component,
        writable,
        array_ref,
        array_length,
        storage_format,
    }
}

fn texture_handle_array(name: &str) -> Option<Option<u32>> {
    let compact = name
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();
    if compact.starts_with("array_ref<texture") || compact.starts_with("array_ref<depth") {
        return Some(None);
    }
    if !compact.starts_with("array<texture") && !compact.starts_with("array<depth") {
        return None;
    }
    let length = compact
        .rfind('>')
        .and_then(|end| compact[..end].rsplit_once(','))
        .and_then(|(_, tail)| tail.parse().ok());
    Some(length)
}

fn storage_format_from_name(name: &str, component: TextureComponent) -> TextureFormat {
    match component {
        TextureComponent::Float => {
            if name.contains("<half") {
                TextureFormat::Rgba16f
            } else if name.contains("<float") {
                TextureFormat::R32f
            } else {
                TextureFormat::Rgba32f
            }
        }
        TextureComponent::Uint => {
            if name.contains("<ushort") {
                TextureFormat::Rgba16ui
            } else {
                TextureFormat::Rgba8ui
            }
        }
        TextureComponent::Sint => {
            if name.contains("<short") {
                TextureFormat::Rgba16i
            } else {
                TextureFormat::Rgba8i
            }
        }
    }
}

fn texture_component_from_name(name: &str) -> TextureComponent {
    let Some((_, rest)) = name.split_once('<') else {
        return TextureComponent::Float;
    };
    let scalar = rest
        .split(|c: char| c == ',' || c == '>' || c.is_whitespace())
        .find(|part| !part.is_empty())
        .unwrap_or("");
    let scalar = scalar.rsplit('<').next().unwrap_or(scalar);
    match scalar {
        "uint" | "ushort" | "uchar" => TextureComponent::Uint,
        "int" | "short" | "char" => TextureComponent::Sint,
        _ => TextureComponent::Float,
    }
}

fn texture_access_from_name(name: &str) -> (bool, bool) {
    let array_ref = texture_handle_array(name).is_some();
    let Some((_, rest)) = name.split_once('<') else {
        return (false, array_ref);
    };
    let Some(inner) = rest.split('>').next() else {
        return (false, array_ref);
    };
    let mut fields = inner.split(',').map(str::trim);
    let _scalar = fields.next();
    let writable = matches!(fields.next(), Some("write") | Some("read_write"));
    (writable, array_ref)
}
