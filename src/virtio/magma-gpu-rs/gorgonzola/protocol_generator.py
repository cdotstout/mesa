# Copyright 2026 The Magma GPU Project
# SPDX-License-Identifier: MIT

from __future__ import annotations
from pathlib import Path
import typing as T

from gorgonzola.base_generator import BaseGenerator
from gorgonzola.common import (
    CopyrightInfo,
    ApiAspectMask,
    GorgonzolaConstant,
    GorgonzolaConstantArray,
    GorgonzolaBaseEnum,
    GorgonzolaFlag,
    GorgonzolaStruct,
    GorgonzolaExtensibleStruct,
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

EXTENSIBLE_HEADER = """\
#[derive(Debug, Default, Clone, Copy, FromBytes, IntoBytes, Immutable)]
#[repr(C, align(8))]
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
        let mut curr = self.header().p_next as usize;
        while curr != 0 {{
            let header = unsafe {{ &*(curr as *const {api}StructureTypeHeader) }};
            if header.stype == T::STRUCTURE_TYPE as u32 {{
                return Some(unsafe {{ &*(curr as *const T) }});
            }}
            curr = header.p_next as usize;
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
        let mut curr = self.header().p_next as usize;
        while curr != 0 {{
            let header = &mut *(curr as *mut {api}StructureTypeHeader);
            if header.stype == T::STRUCTURE_TYPE as u32 {{
                return Some(&mut *(curr as *mut T));
            }}
            curr = header.p_next as usize;
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
        while ((*curr).p_next as usize) != 0 {{
            curr = ((*curr).p_next as usize) as *mut {api}StructureTypeHeader;
        }}
        (*curr).p_next = (next.header_mut() as *mut {api}StructureTypeHeader) as usize as u64;
    }}
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


class ProtocolGenerator(BaseGenerator):
    ASPECT_MASK = ApiAspectMask.WIRE

    def __init__(
        self,
        full_path: Path,
        copyright_info: CopyrightInfo,
        api_name: str,
        api_data: PostProcessedApiData,
    ) -> None:
        super().__init__(full_path, copyright_info, api_name, api_data)
        self._extensible_preamble_emitted = False
        self.ext_variants: T.List[str] = []
        self.req_variants: T.List[str] = []
        self.resp_variants: T.List[str] = []

    def _emit_layout_entries(
        self, obj: GorgonzolaStruct, is_header_struct: bool = False
    ) -> None:
        members_by_name = {m.name: m for m in obj.members}
        if obj.protocol_layout:
            for e in obj.protocol_layout.entries:
                type_name = e.type_name
                if is_header_struct and not e.is_padding:
                    m = members_by_name.get(e.name)
                    if (
                        m is not None
                        and isinstance(m.type, GorgonzolaStruct)
                        and m.type.has_dynamic_array
                    ):
                        type_name = f'{type_name}Header'
                self.code_writer.write_line(f'pub {e.name}: {type_name},')
        if not is_header_struct:
            for m in obj.dynamic_array_members:
                self.code_writer.write_line(
                    f'pub {m.name}: {to_rust_protocol_type(m.type)},'
                )

    def _emit_opcodes(self) -> None:
        api_upper = self.api_name.pascal.upper()
        req_opcodes = []
        resp_opcodes = []
        for cmd in self.api_data.commands_for_aspect(self.ASPECT_MASK):
            assert cmd.req_struct is not None
            cmd_snake = to_snake_case(cmd.req_struct.name).upper()
            req_opcodes.append(f'pub const {api_upper}_OPCODE_{cmd_snake}: u32 = {cmd.opcode};')
            if cmd.response:
                val = 0x80000000 | int(cmd.opcode)
                hex_str = f'{val:08x}'
                resp_opcode = f'0x{hex_str[:4]}_{hex_str[4:]}'
                resp_opcodes.append(
                    f'pub const {api_upper}_OPCODE_RESP_{cmd_snake}: u32 = {resp_opcode};'
                )
        if req_opcodes:
            w = self.code_writer
            w.write_line()
            w.write_lines(req_opcodes)
            if resp_opcodes:
                w.write_line()
                w.write_lines(resp_opcodes)

    def emit_constant(self, obj: GorgonzolaConstant) -> None:
        self.code_writer.write_line(f'pub const {obj.name}: {obj.type_info.name} = {obj.value};')

    def emit_enum(self, obj: GorgonzolaBaseEnum) -> None:
        w = self.code_writer
        w.write_line()
        w.write_line(
            '#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, TryFromBytes, IntoBytes, Immutable)]'
        )
        w.write_line(f'#[repr({obj.type_info.name})]')
        w.write_line(f'pub enum {obj.name} {{')
        with w.indent():
            for i, entry in enumerate(obj.entries):
                if i == 0:
                    w.write_line('#[default]')
                w.write_line(f'{entry.name} = {entry.value},')
        w.write_line('}')

    def emit_flag(self, obj: GorgonzolaFlag) -> None:
        rust_type = obj.type_info.name
        w = self.code_writer
        w.write_line()
        w.write_raw(FLAG_PROPERTIES.rstrip('\n'))
        w.write_line(f'pub struct {obj.name}({rust_type});')
        w.write_line('bitflags! {')
        with w.indent():
            w.write_line(f'impl {obj.name}: {rust_type} {{')
            with w.indent():
                for entry in obj.entries:
                    w.write_line(f'const {entry.name} = {entry.value};')
            w.write_line('}')
        w.write_line('}')

    def _has_large_array(self, obj: GorgonzolaStruct) -> bool:
        return any(
            isinstance(m.type, GorgonzolaConstantArray) and m.type.array_size > 32
            for m in obj.members
        )

    def _emit_manual_default(self, obj: GorgonzolaStruct, has_header: bool = False) -> None:
        w = self.code_writer
        w.write_line()
        w.write_line(f'impl Default for {obj.name} {{')
        with w.indent():
            w.write_fn('default', [], ret='Self', vis='')
            with w.indent():
                w.write_line('Self {')
                with w.indent():
                    if has_header:
                        w.write_line('header: Default::default(),')
                    if obj.protocol_layout:
                        for e in obj.protocol_layout.entries:
                            if e.type_name.startswith('['):
                                size_expr = e.type_name.split(';')[1].rstrip(']').strip()
                                w.write_line(f'{e.name}: [Default::default(); {size_expr}],')
                            else:
                                w.write_line(f'{e.name}: Default::default(),')
                w.write_line('}')
            w.write_line('}')
        w.write_line('}')

    def emit_struct(self, obj: GorgonzolaStruct) -> None:
        w = self.code_writer
        w.write_line()
        has_large_array = self._has_large_array(obj)
        if obj.has_dynamic_array:
            if has_large_array:
                w.write_line('#[derive(Debug, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]')
            else:
                w.write_line(
                    '#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]'
                )
            w.write_line('#[repr(C)]')
            w.write_line(f'pub struct {obj.name}Header {{')
            with w.indent():
                self._emit_layout_entries(obj, is_header_struct=True)
            w.write_line('}')
            w.write_line()
            w.write_line('#[derive(Debug, Default, Clone)]')
            w.write_line('#[repr(C)]')
            w.write_line(f'pub struct {obj.name} {{')
            with w.indent():
                self._emit_layout_entries(obj, is_header_struct=False)
            w.write_line('}')
            return

        if has_large_array:
            w.write_line('#[derive(Debug, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]')
        else:
            w.write_line('#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]')
        w.write_line('#[repr(C)]')
        w.write_line(f'pub struct {obj.name} {{')
        with w.indent():
            self._emit_layout_entries(obj)
        w.write_line('}')
        if has_large_array:
            self._emit_manual_default(obj, has_header=False)

    def _emit_extensible_preamble(self) -> None:
        stype_enum = self.api_data.structure_type_enum_for_aspect(self.ASPECT_MASK)
        if not stype_enum:
            return
        api = self.api_name.pascal
        api_upper = api.upper()
        w = self.code_writer
        w.write_line()
        for entry in stype_enum.entries:
            stype_name = to_snake_case(entry.name).upper()
            w.write_line(f'pub const {api_upper}_STRUCTURE_TYPE_{stype_name}: u32 = {entry.value};')
        w.write_line()
        w.write_line('#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromBytes, IntoBytes, Immutable)]')
        w.write_line('#[repr(u32)]')
        w.write_line(f'pub enum {api}StructureType {{')
        with w.indent():
            for entry in stype_enum.entries:
                w.write_line(f'{entry.name} = {entry.value},')
        w.write_line('}')
        w.write_line()
        w.write_raw(EXTENSIBLE_HEADER.format(api=api).rstrip('\n'))
        w.write_line()
        w.write_raw(EXTENSIBLE_TRAITS.format(api=api).rstrip('\n'))
        w.write_line()

    def emit_extensible_struct(self, obj: GorgonzolaExtensibleStruct) -> None:
        if not self._extensible_preamble_emitted:
            self._emit_extensible_preamble()
            self._extensible_preamble_emitted = True

        api = self.api_name.pascal
        w = self.code_writer
        has_large_array = self._has_large_array(obj)
        if obj.has_dynamic_array:
            w.write_line('#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]')
            w.write_line('#[repr(C)]')
            w.write_line(f'pub struct {obj.name}Header {{')
            with w.indent():
                w.write_line(f'pub header: {api}StructureTypeHeader,')
                self._emit_layout_entries(obj, is_header_struct=True)
            w.write_line('}')
            w.write_line()
            w.write_line('#[derive(Debug, Default, Clone)]')
            w.write_line('#[repr(C)]')
            w.write_line(f'pub struct {obj.name} {{')
            with w.indent():
                w.write_line(f'pub header: {api}StructureTypeHeader,')
                self._emit_layout_entries(obj, is_header_struct=False)
            w.write_line('}')
        else:
            if has_large_array:
                w.write_line('#[derive(Debug, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]')
            else:
                w.write_line(
                    '#[derive(Debug, Default, Clone, Copy, TryFromBytes, IntoBytes, Immutable)]'
                )
            w.write_line('#[repr(C)]')
            w.write_line(f'pub struct {obj.name} {{')
            with w.indent():
                w.write_line(f'pub header: {api}StructureTypeHeader,')
                self._emit_layout_entries(obj)
            w.write_line('}')
            if has_large_array:
                self._emit_manual_default(obj, has_header=True)
        w.write_line()
        w.write_line(f'unsafe impl {api}TaggedStructure for {obj.name} {{')
        with w.indent():
            const_line = (
                f'const STRUCTURE_TYPE: {api}StructureType = {api}StructureType::{obj.stype};'
            )
            if len(const_line) + w.INDENT_SIZE > w.MAX_LINE_LENGTH:
                w.write_line(f'const STRUCTURE_TYPE: {api}StructureType =')
                with w.indent():
                    w.write_line(f'{api}StructureType::{obj.stype};')
            else:
                w.write_line(const_line)
            w.write_line()
            w.write_fn('header', ['&self'], ret=f'&{api}StructureTypeHeader', vis='')
            with w.indent():
                w.write_line('&self.header')
            w.write_line('}')
            w.write_line()
            w.write_fn('header_mut', ['&mut self'], ret=f'&mut {api}StructureTypeHeader', vis='')
            with w.indent():
                w.write_line('&mut self.header')
            w.write_line('}')
        w.write_line('}')
        if obj.extends is not None:
            w.write_line(f'unsafe impl {api}Extends<{obj.extends.name}> for {obj.name} {{}}')

        variant_name = normalize_command_name(obj.name, api)
        self.ext_variants.append(f'{variant_name}({obj.name})')

    def _emit_extensible_enum(self) -> None:
        if not self.ext_variants:
            return
        w = self.code_writer
        w.write_line('#[allow(clippy::large_enum_variant)]')
        w.write_line('#[derive(Debug)]')
        w.write_line(f'pub enum {self.api_name.pascal}ExtensibleStruct {{')
        with w.indent():
            for v in self.ext_variants:
                w.write_line(f'{v},')
        w.write_line('}')

    def _emit_command_struct(self, name: str, struct: GorgonzolaStruct) -> None:
        w = self.code_writer
        w.write_line()
        if struct.has_dynamic_array:
            w.write_line('#[repr(C)]')
            w.write_line('#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]')
            w.write_line(f'pub struct {name}Header {{')
            with w.indent():
                w.write_line(f'pub header: {self.api_name.pascal}CommandHeader,')
                self._emit_layout_entries(struct, is_header_struct=True)
            w.write_line('}')
            w.write_line()
            w.write_line('#[derive(Debug, Default, Clone)]')
            w.write_line(f'pub struct {name} {{')
            with w.indent():
                w.write_line(f'pub header: {self.api_name.pascal}CommandHeader,')
                self._emit_layout_entries(struct, is_header_struct=False)
            w.write_line('}')
            return

        w.write_line('#[repr(C)]')
        w.write_line('#[derive(Debug, Default, Copy, Clone, TryFromBytes, IntoBytes, Immutable)]')
        w.write_line(f'pub struct {name} {{')
        with w.indent():
            w.write_line(f'pub header: {self.api_name.pascal}CommandHeader,')
            self._emit_layout_entries(struct)
        w.write_line('}')

    def emit_command(self, obj: GorgonzolaCommand) -> None:
        assert obj.req_struct is not None
        req_name = obj.req_struct.name
        self._emit_command_struct(req_name, obj.req_struct)
        self.req_variants.append(f'{req_name}({req_name})')

        if obj.resp_struct:
            resp_name = obj.resp_struct.name
            self._emit_command_struct(resp_name, obj.resp_struct)
            self.resp_variants.append(f'{resp_name}({resp_name})')

    def _emit_protocol_enum(self) -> None:
        if not self.req_variants:
            return
        w = self.code_writer
        w.write_line()
        w.write_line('#[allow(clippy::large_enum_variant)]')
        w.write_line('#[derive(Debug)]')
        w.write_line(f'pub enum {self.api_name.pascal}Protocol {{')
        with w.indent():
            for v in self.req_variants:
                w.write_line(f'{v},')
            if self.resp_variants:
                w.write_line()
                for v in self.resp_variants:
                    w.write_line(f'{v},')
        w.write_line('}')

    def _emit_internal(self) -> None:
        w = self.code_writer
        self.emit_license()
        w.write_line()
        w.write_raw(COMMON_RUST_IMPORTS.rstrip('\n'))

        self._emit_opcodes()
        if any(self.api_data.constants(self.ASPECT_MASK)):
            w.write_line()
        self.dispatch_all_types()
        self._emit_extensible_enum()

        if any(self.api_data.commands_for_aspect(self.ASPECT_MASK)):
            w.write_line()
            w.write_raw(COMMAND_HEADER.format(api=self.api_name.pascal).rstrip('\n'))
            self.dispatch_all_commands()
            self._emit_protocol_enum()

