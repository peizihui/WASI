use crate::ast::*;
use std::collections::HashMap;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SizeAlign {
    pub size: usize,
    pub align: usize,
}

pub trait Layout {
    fn mem_size_align(&self) -> SizeAlign;
    fn mem_size(&self) -> usize {
        self.mem_size_align().size
    }
    fn mem_align(&self) -> usize {
        self.mem_size_align().align
    }
}

impl TypeRef {
    fn layout(&self, cache: &mut HashMap<TypeRef, SizeAlign>) -> SizeAlign {
        if let Some(hit) = cache.get(self) {
            return *hit;
        }
        let layout = match &*self.type_() {
            Type::Enum(e) => e.repr.mem_size_align(),
            Type::Int(i) => i.repr.mem_size_align(),
            Type::Flags(f) => f.repr.mem_size_align(),
            Type::Struct(s) => s.layout(cache),
            Type::Union(u) => u.layout(cache),
            Type::Handle(h) => h.mem_size_align(),
            Type::Array { .. } => BuiltinType::String.mem_size_align(),
            Type::Pointer { .. } | Type::ConstPointer { .. } => BuiltinType::U32.mem_size_align(),
            Type::Builtin(b) => b.mem_size_align(),
        };
        cache.insert(self.clone(), layout);
        layout
    }
}

impl Layout for TypeRef {
    fn mem_size_align(&self) -> SizeAlign {
        let mut cache = HashMap::new();
        self.layout(&mut cache)
    }
}
impl Layout for IntRepr {
    fn mem_size_align(&self) -> SizeAlign {
        match self {
            IntRepr::U8 => BuiltinType::U8.mem_size_align(),
            IntRepr::U16 => BuiltinType::U16.mem_size_align(),
            IntRepr::U32 => BuiltinType::U32.mem_size_align(),
            IntRepr::U64 => BuiltinType::U64.mem_size_align(),
        }
    }
}

pub struct StructMemberLayout<'a> {
    pub member: &'a StructMember,
    pub offset: usize,
}

impl StructDatatype {
    pub fn member_layout(&self) -> Vec<StructMemberLayout> {
        self.member_layout_(&mut HashMap::new())
    }

    fn member_layout_(&self, cache: &mut HashMap<TypeRef, SizeAlign>) -> Vec<StructMemberLayout> {
        let mut members = Vec::new();
        let mut offset = 0;
        for m in self.members.iter() {
            let sa = m.tref.layout(cache);
            offset = align_to(offset, sa.align);
            members.push(StructMemberLayout { member: m, offset });
            offset += sa.size;
        }
        members
    }

    fn layout(&self, cache: &mut HashMap<TypeRef, SizeAlign>) -> SizeAlign {
        let members = self.member_layout_(cache);
        let align = members
            .iter()
            .map(|m| m.member.tref.layout(cache).align)
            .max()
            .expect("nonzero struct members");
        let last = members.last().expect("nonzero struct members");
        let size = last.offset + last.member.tref.layout(cache).size;
        let size = align_to(size, align);
        SizeAlign { size, align }
    }
}

impl Layout for StructDatatype {
    fn mem_size_align(&self) -> SizeAlign {
        let mut cache = HashMap::new();
        self.layout(&mut cache)
    }
}

/// If the next free byte in the struct is `offs`, and the next
/// element has alignment `alignment`, determine the offset at
/// which to place that element.
fn align_to(offs: usize, alignment: usize) -> usize {
    offs + alignment - 1 - ((offs + alignment - 1) % alignment)
}

#[cfg(test)]
mod test {
    use super::align_to;
    #[test]
    fn align() {
        assert_eq!(0, align_to(0, 1));
        assert_eq!(0, align_to(0, 2));
        assert_eq!(0, align_to(0, 4));
        assert_eq!(0, align_to(0, 8));

        assert_eq!(1, align_to(1, 1));
        assert_eq!(2, align_to(1, 2));
        assert_eq!(4, align_to(1, 4));
        assert_eq!(8, align_to(1, 8));

        assert_eq!(2, align_to(2, 1));
        assert_eq!(2, align_to(2, 2));
        assert_eq!(4, align_to(2, 4));
        assert_eq!(8, align_to(2, 8));

        assert_eq!(5, align_to(5, 1));
        assert_eq!(6, align_to(5, 2));
        assert_eq!(8, align_to(5, 4));
        assert_eq!(8, align_to(5, 8));
    }
}

impl UnionDatatype {
    fn layout(&self, cache: &mut HashMap<TypeRef, SizeAlign>) -> SizeAlign {
        let sas = self
            .variants
            .iter()
            .map(|v| v.tref.layout(cache))
            .collect::<Vec<SizeAlign>>();
        let size = sas
            .iter()
            .map(|sa| sa.size)
            .max()
            .expect("nonzero variants");
        let align = sas
            .iter()
            .map(|sa| sa.align)
            .max()
            .expect("nonzero variants");
        let size = align_to(size, align);
        SizeAlign { size, align }
    }
}

impl Layout for UnionDatatype {
    fn mem_size_align(&self) -> SizeAlign {
        let mut cache = HashMap::new();
        self.layout(&mut cache)
    }
}

impl Layout for HandleDatatype {
    fn mem_size_align(&self) -> SizeAlign {
        BuiltinType::U32.mem_size_align()
    }
}

impl Layout for BuiltinType {
    fn mem_size_align(&self) -> SizeAlign {
        match self {
            BuiltinType::String => SizeAlign { size: 8, align: 4 }, // Pointer and Length
            BuiltinType::U8 | BuiltinType::S8 | BuiltinType::Char8 => {
                SizeAlign { size: 1, align: 1 }
            }
            BuiltinType::U16 | BuiltinType::S16 => SizeAlign { size: 2, align: 2 },
            BuiltinType::USize | BuiltinType::U32 | BuiltinType::S32 | BuiltinType::F32 => {
                SizeAlign { size: 4, align: 4 }
            }
            BuiltinType::U64 | BuiltinType::S64 | BuiltinType::F64 => {
                SizeAlign { size: 8, align: 8 }
            }
        }
    }
}
