# Copyright 2026 The Magma GPU Project
# SPDX-License-Identifier: MIT

from __future__ import annotations
import typing as T

from gorgonzola.common import (
    SINGLE_INDENT,
    CommonGenerator,
    ApiAspectMask,
    GorgonzolaConstant,
    GorgonzolaEnum,
    GorgonzolaFlag,
    GorgonzolaStruct,
    GorgonzolaExtensibleStruct,
    GorgonzolaStructMember,
    GorgonzolaCommand,
    to_rust_protocol_type,
)
from gorgonzola.post_process import PostProcessedApiData
from gorgonzola.utils import normalize_command_name, to_snake_case

COMMON_RUST_IMPORTS = """\
#![allow(unused_imports)]
#![allow(dead_code)]

use bitflags::bitflags;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, TryFromBytes};
"""

FLAG_PROPERTIES = """\
#[derive(
    Copy, Clone, Default, Debug, PartialEq, Eq, IntoBytes, FromBytes, Immutable, KnownLayout,
)]
#[repr(transparent)]
"""

ENUM = """\
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, TryFromBytes, IntoBytes, Immutable)]
#[repr({type_name})]
pub enum {name} {{
{entries}}}

"""

STRUCT = """\
#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct {name} {{
{members}
}}

"""

EXTENSIBLE_STYPE_ENUM = """\
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromBytes, IntoBytes, Immutable)]
#[repr(u32)]
pub enum {api}StructureType {{
{entries}
}}

"""

BITFLAGS_IMPL = """\
bitflags! {{
    {decl}
}}

"""


EXTENSIBLE_HEADER = """\
#[derive(Debug, Default, Clone, Copy, FromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct {api}StructureTypeHeader {{
    pub stype: u32,
    pub size: u32,
    pub p_next: u64,
}}

"""

EXTENSIBLE_TRAITS = """\
/// Marker trait indicating that structure `Self` is allowed to extend structure `Root`.
///
/// # Safety
/// Implementors must only be declared when the specification allows `Self` to extend `Root`.
pub unsafe trait {api}Extends<Root> {{}}

/// Trait implemented by extensible structures indicating their structure type.
///
/// # Safety
/// Implementors must guarantee that `STRUCTURE_TYPE` matches the actual header's stype
/// and that the structure begins with a valid `{api}StructureTypeHeader`.
pub unsafe trait {api}TaggedStructure {{
    const STRUCTURE_TYPE: {api}StructureType;
    fn header(&self) -> &{api}StructureTypeHeader;
    fn header_mut(&mut self) -> &mut {api}StructureTypeHeader;

    /// Find an extension in the `p_next` chain matching type `T`.
    fn get_extension<T: {api}TaggedStructure + {api}Extends<Self>>(&self) -> Option<&T>
    where
        Self: Sized,
    {{
        let mut curr = self.header().p_next;
        while curr != 0 {{
            let header = unsafe {{ &*(curr as *const {api}StructureTypeHeader) }};
            if header.stype == T::STRUCTURE_TYPE as u32 {{
                return Some(unsafe {{ &*(curr as *const T) }});
            }}
            curr = header.p_next;
        }}
        None
    }}

    /// Find a mutable extension in the `p_next` chain matching type `T`.
    ///
    /// # Safety
    /// Caller must ensure exclusive access to the structures in the chain.
    unsafe fn get_extension_mut<T: {api}TaggedStructure + {api}Extends<Self>>(
        &mut self,
    ) -> Option<&mut T>
    where
        Self: Sized,
    {{
        let mut curr = self.header().p_next;
        while curr != 0 {{
            let header = &mut *(curr as *mut {api}StructureTypeHeader);
            if header.stype == T::STRUCTURE_TYPE as u32 {{
                return Some(&mut *(curr as *mut T));
            }}
            curr = header.p_next;
        }}
        None
    }}

    /// Append an extension structure to the end of the `p_next` chain.
    ///
    /// # Safety
    /// `next` must point to a valid extensible structure whose memory remains valid
    /// while the chain is referenced.
    unsafe fn push_next<T: {api}TaggedStructure + {api}Extends<Self>>(&mut self, next: &mut T)
    where
        Self: Sized,
    {{
        let mut curr: *mut {api}StructureTypeHeader = self.header_mut();
        while (*curr).p_next != 0 {{
            curr = (*curr).p_next as *mut {api}StructureTypeHeader;
        }}
        (*curr).p_next = (next.header_mut() as *mut {api}StructureTypeHeader) as usize as u64;
    }}
}}

"""

TAGGED_IMPL = """\
unsafe impl {api}TaggedStructure for {name} {{
{const_decl}

    fn header(&self) -> &{api}StructureTypeHeader {{
        &self.header
    }}

    fn header_mut(&mut self) -> &mut {api}StructureTypeHeader {{
        &mut self.header
    }}
}}
"""

EXTENDS_IMPL = """\
unsafe impl {api}Extends<{root}> for {name} {{}}
"""

EXTENSIBLE_ENUM = """\
#[derive(Debug)]
pub enum {api}ExtensibleStruct {{
{variants}
}}

"""

EXTENSIBLE_STRUCT = """\
#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]
#[repr(C)]
pub struct {name} {{
    pub header: {api}StructureTypeHeader,
{members}
}}

"""

PROTOCOL_ENUM = """\
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum {api}Protocol {{
{variants}
}}

"""

COMMAND_HEADER = """\
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, FromBytes, IntoBytes, Immutable)]
pub struct {api}CommandHeader {{
    pub opcode: u32,
    pub size: u32,
}}

"""

PROTOCOL_COMMAND_STRUCT = """\
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]
pub struct {name}{suffix} {{
    pub header: {api}CommandHeader,{members}
}}

"""


class ProtocolGenerator(CommonGenerator):
    ASPECT_MASK = ApiAspectMask.WIRE

    def _format_members(self, obj: GorgonzolaStruct) -> str:
        if obj.protocol_layout:
            return '\n'.join(
                f'{SINGLE_INDENT}pub {e.name}: {e.type_name},'
                for e in obj.protocol_layout.entries
            )
        return ''

    def _format_command_members(self, struct: GorgonzolaStruct) -> str:
        if not struct.protocol_layout or not struct.protocol_layout.entries:
            return ''
        return '\n' + '\n'.join(
            f'{SINGLE_INDENT}pub {e.name}: {e.type_name},'
            for e in struct.protocol_layout.entries
        )

    def _emit_opcodes(self) -> None:
        api = self.api_name.pascal
        api_upper = api.upper()
        req_opcodes = []
        resp_opcodes = []
        for cmd in self.api_data.commands_for_aspect(self.ASPECT_MASK):
            cmd_pascal = normalize_command_name(cmd.name, api)
            cmd_snake = to_snake_case(cmd_pascal).upper()
            req_opcodes.append(f'pub const {api_upper}_OPCODE_{cmd_snake}: u32 = {cmd.opcode};\n')
            if cmd.response:
                val = 0x80000000 | int(cmd.opcode)
                hex_str = f'{val:08x}'
                resp_opcode = f'0x{hex_str[:4]}_{hex_str[4:]}'
                resp_opcodes.append(
                    f'pub const {api_upper}_OPCODE_RESP_{cmd_snake}: u32 = {resp_opcode};\n'
                )
        if req_opcodes:
            opcodes_str = ''.join(req_opcodes)
            if resp_opcodes:
                opcodes_str += '\n' + ''.join(resp_opcodes)
            self.code_writer.write_line()
            self.code_writer.write_raw(opcodes_str.strip())

    def _emit_constants(self) -> None:
        const_lines = []
        for obj in self.api_data.constants(self.ASPECT_MASK):
            rust_type = to_rust_protocol_type(self.api_data.resolve_type_name(obj.type_name))
            const_lines.append(f'pub const {obj.name}: {rust_type} = {obj.value};\n')
        if const_lines:
            self.code_writer.write_line()
            self.code_writer.write_raw(''.join(const_lines).strip())

    def _emit_enums(self) -> None:
        enum_defs = []
        for obj in self.api_data.enums(self.ASPECT_MASK):
            rust_type = obj.type_info.name
            entries = ''
            for i, entry in enumerate(obj.entries):
                if i == 0:
                    entries += f'{SINGLE_INDENT}#[default]\n'
                entries += f'{SINGLE_INDENT}{entry.name} = {entry.value},\n'
            enum_defs.append(ENUM.format(name=obj.name, type_name=rust_type, entries=entries))
        if enum_defs:
            self.code_writer.write_line()
            self.code_writer.write_raw(''.join(enum_defs).strip())

    def _emit_flags(self) -> None:
        flag_defs = []
        for obj in self.api_data.flags(self.ASPECT_MASK):
            rust_type = obj.type_info.name
            flag_str = FLAG_PROPERTIES + f'pub struct {obj.name}({rust_type});\n'
            bitflags_decl = f'impl {obj.name}: {rust_type} {{\n'
            for entry in obj.entries:
                bitflags_decl += f'        const {entry.name} = {entry.value};\n'
            bitflags_decl += f'{SINGLE_INDENT}}}'
            flag_str += BITFLAGS_IMPL.format(decl=bitflags_decl)
            flag_defs.append(flag_str)
        if flag_defs:
            self.code_writer.write_line()
            self.code_writer.write_raw(''.join(flag_defs).strip())

    def _emit_structs(self) -> None:
        struct_defs = []
        for obj in self.api_data.structs(self.ASPECT_MASK):
            members_val = self._format_members(obj)
            struct_defs.append(STRUCT.format(name=obj.name, members=members_val))
        if struct_defs:
            self.code_writer.write_line()
            self.code_writer.write_raw(''.join(struct_defs).strip())

    def _emit_extensible_section(self) -> None:
        ext_structs = list(self.api_data.extensible_structs_for_aspect(self.ASPECT_MASK))
        if not ext_structs:
            return
        stype_enum = self.api_data.structure_type_enum_for_aspect(self.ASPECT_MASK)
        if not stype_enum:
            return

        api = self.api_name.pascal
        api_upper = api.upper()

        stype_consts = ''
        stype_entries = ''
        for entry in stype_enum.entries:
            stype_name = to_snake_case(entry.name).upper()
            stype_consts += f'pub const {api_upper}_STRUCTURE_TYPE_{stype_name}: u32 = {entry.value};\n'
            stype_entries += f'{SINGLE_INDENT}{entry.name} = {entry.value},\n'

        res = stype_consts + '\n' if stype_consts else ''
        res += EXTENSIBLE_STYPE_ENUM.format(api=api, entries=stype_entries.rstrip())
        res += EXTENSIBLE_HEADER.format(api=api)
        res += EXTENSIBLE_TRAITS.format(api=api)

        ext_variants = []
        for obj in ext_structs:
            members_val = self._format_members(obj)
            res += EXTENSIBLE_STRUCT.format(api=api, name=obj.name, members=members_val)
            variant_name = normalize_command_name(obj.name, api)
            ext_variants.append(f'{SINGLE_INDENT}{variant_name}({obj.name}),\n')

            const_line = f"    const STRUCTURE_TYPE: {api}StructureType = {api}StructureType::{obj.stype};"
            if len(const_line) > 100:
                const_decl = (
                    f"    const STRUCTURE_TYPE: {api}StructureType =\n"
                    f"        {api}StructureType::{obj.stype};"
                )
            else:
                const_decl = const_line

            trait_impl = TAGGED_IMPL.format(api=api, name=obj.name, const_decl=const_decl)
            if obj.extends:
                roots = [r.strip() for r in obj.extends.split(',') if r.strip()]
                for root in roots:
                    trait_impl += EXTENDS_IMPL.format(api=api, root=root, name=obj.name)
            res += trait_impl

        res += EXTENSIBLE_ENUM.format(api=api, variants=''.join(ext_variants).rstrip())
        self.code_writer.write_line()
        self.code_writer.write_raw(res.strip())

    def _emit_commands_section(self) -> None:
        commands = list(self.api_data.commands_for_aspect(self.ASPECT_MASK))
        if not commands:
            return
        api = self.api_name.pascal
        command_structs = []
        req_variants = []
        resp_variants = []

        for obj in commands:
            cmd_pascal = normalize_command_name(obj.name, api)
            req_variants.append(f'{SINGLE_INDENT}{cmd_pascal}({cmd_pascal}),\n')
            assert obj.req_struct is not None
            req_members_str = self._format_command_members(obj.req_struct)
            command_structs.append(
                PROTOCOL_COMMAND_STRUCT.format(
                    api=api, name=cmd_pascal, suffix='', members=req_members_str
                )
            )

            if obj.resp_struct:
                resp_variants.append(f'{SINGLE_INDENT}{cmd_pascal}Resp({cmd_pascal}Resp),\n')
                resp_members_str = self._format_command_members(obj.resp_struct)
                command_structs.append(
                    PROTOCOL_COMMAND_STRUCT.format(
                        api=api, name=cmd_pascal, suffix='Resp', members=resp_members_str
                    )
                )

        variants_str = ''.join(req_variants)
        if resp_variants:
            variants_str += '\n' + ''.join(resp_variants)
        protocol_enum = PROTOCOL_ENUM.format(api=api, variants=variants_str.rstrip())

        res = COMMAND_HEADER.format(api=api) + ''.join(command_structs) + protocol_enum
        self.code_writer.write_line()
        self.code_writer.write_raw(res.strip())

    def _emit_internal(self) -> None:
        self.code_writer.write_raw(self._license().rstrip('\n'))
        self.code_writer.write_line()
        self.code_writer.write_raw(COMMON_RUST_IMPORTS.rstrip('\n'))

        self._emit_opcodes()
        self._emit_constants()
        self._emit_enums()
        self._emit_flags()
        self._emit_structs()
        self._emit_extensible_section()
        self._emit_commands_section()


class RustApiGenerator(CommonGenerator):
    ASPECT_MASK = ApiAspectMask.RUST_API

    def emit_struct(self, obj: GorgonzolaStruct) -> None:
        struct_name = obj.name
        w = self.code_writer
        ext_struct = obj if isinstance(obj, GorgonzolaExtensibleStruct) else None
        arg_lines = [f'{m.name}: {to_rust_protocol_type(m.type)}' for m in obj.members]

        w.write_line(f'impl {struct_name} {{')
        w.set_indent_multiple(1)

        w.write_fn('new', arg_lines, ret=struct_name)
        w.set_indent_multiple(2)

        header_lines = None
        if ext_struct is not None:
            api = self.api_name.pascal
            header_lines = [
                f'header: {api}StructureTypeHeader {{',
                f'    stype: {api}StructureType::{ext_struct.stype} as u32,',
                f'    size: std::mem::size_of::<{struct_name}>() as u32,',
                f'    p_next: 0,',
                f'}},',
            ]

        if obj.protocol_layout:
            all_field_names = {e.name for e in obj.protocol_layout.entries}
        else:
            all_field_names = {m.name for m in obj.members}
        if ext_struct is not None:
            all_field_names.add('header')

        assigned_names = {m.name for m in obj.members}
        if ext_struct is not None:
            assigned_names.add('header')

        has_default = bool(all_field_names - assigned_names)
        member_names = [m.name for m in obj.members]

        w.write_struct_instantiation(
            struct_name, member_names, has_default=has_default, header_lines=header_lines
        )

        w.set_indent_multiple(1)
        w.write_line('}')

        if ext_struct is not None:
            extensions = self.api_data.extensions_for(obj.name)
            for ext in extensions:
                ext_name = ext.name
                getter_name = ext.rust_member_name or to_snake_case(ext.stype)
                w.write_line()
                w.write_fn(getter_name, ['&self'], ret=f'Option<&{ext_name}>')
                w.set_indent_multiple(2)
                w.write_line(f'self.get_extension::<{ext_name}>()')
                w.set_indent_multiple(1)
                w.write_line('}')
                w.write_line()
                w.write_fn(f'{getter_name}_mut', ['&mut self'], ret=f'Option<&mut {ext_name}>')
                w.set_indent_multiple(2)
                w.write_line(f'unsafe {{ self.get_extension_mut::<{ext_name}>() }}')
                w.set_indent_multiple(1)
                w.write_line('}')

        w.set_indent_multiple(0)
        w.write_line('}')
        w.write_line()

    def _emit_internal(self) -> None:
        self.code_writer.write_raw(self._license().rstrip('\n'))
        self.code_writer.write_line()
        self.code_writer.write_line('use crate::protocol::*;')
        self.code_writer.write_line()

        for t in self.api_data.types(self.ASPECT_MASK):
            self.dispatch_type(t)

