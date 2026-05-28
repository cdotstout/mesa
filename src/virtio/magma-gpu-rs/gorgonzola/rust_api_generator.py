# Copyright 2026 The Magma GPU Project
# SPDX-License-Identifier: MIT

from __future__ import annotations

from gorgonzola.base_generator import BaseGenerator
from gorgonzola.common import (
    ApiAspectMask,
    GorgonzolaStruct,
    GorgonzolaExtensibleStruct,
    to_rust_protocol_type,
)
from gorgonzola.utils import to_snake_case


class RustApiGenerator(BaseGenerator):
    ASPECT_MASK = ApiAspectMask.RUST_API

    def _emit_struct_new(self, obj: GorgonzolaStruct) -> None:
        struct_name = obj.name
        w = self.code_writer
        ext_struct = obj if isinstance(obj, GorgonzolaExtensibleStruct) else None
        arg_lines = [f'{m.name}: {to_rust_protocol_type(m.type)}' for m in obj.members]

        w.write_fn('new', arg_lines, ret=struct_name)
        with w.indent():
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
                all_field_names.update(m.name for m in obj.dynamic_array_members)
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
        w.write_line('}')

    def _emit_extension_getters(self, ext_struct: GorgonzolaExtensibleStruct) -> None:
        w = self.code_writer
        for ext in self.api_data.extensions_for(ext_struct.name):
            ext_name = ext.name
            getter_name = ext.rust_member_name or to_snake_case(ext.stype)
            w.write_line()
            w.write_fn(getter_name, ['&self'], ret=f'Option<&{ext_name}>')
            with w.indent():
                w.write_line(f'self.get_extension::<{ext_name}>()')
            w.write_line('}')
            w.write_line()
            w.write_fn(f'{getter_name}_mut', ['&mut self'], ret=f'Option<&mut {ext_name}>')
            with w.indent():
                w.write_line(f'unsafe {{ self.get_extension_mut::<{ext_name}>() }}')
            w.write_line('}')

    def emit_struct(self, obj: GorgonzolaStruct) -> None:
        w = self.code_writer
        w.write_line(f'impl {obj.name} {{')
        with w.indent():
            self._emit_struct_new(obj)
            if isinstance(obj, GorgonzolaExtensibleStruct):
                self._emit_extension_getters(obj)
        w.write_line('}')
        w.write_line()

    def _emit_internal(self) -> None:
        self.emit_license()
        self.code_writer.write_line()
        self.code_writer.write_line('use crate::protocol::*;')
        self.code_writer.write_line()

        self.dispatch_all_types()
