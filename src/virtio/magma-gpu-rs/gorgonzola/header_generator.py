# Copyright 2026 The Magma GPU Project
# SPDX-License-Identifier: MIT

from __future__ import annotations
from pathlib import Path
import typing as T

from gorgonzola.base_generator import BaseGenerator
from gorgonzola.code_writer import MAX_LINE_LENGTH
from gorgonzola.common import (
    CopyrightInfo,
    ApiAspectMask,
    Direction,
    GorgonzolaConstant,
    GorgonzolaBaseEnum,
    GorgonzolaApiObject,
    GorgonzolaConstantArray,
    GorgonzolaDynamicArray,
    GorgonzolaStruct,
    GorgonzolaExtensibleStruct,
    GorgonzolaStructMember,
    GorgonzolaCommand,
    GorgonzolaTypeInstance,
    BaseRustTypeInfo,
    LayoutEntry,
    U8_BYTE_SIZE,
    U16_BYTE_SIZE,
    U32_BYTE_SIZE,
    U64_BYTE_SIZE,
)
from gorgonzola.post_process import PostProcessedApiData
from gorgonzola.utils import (
    create_c_type,
    to_prefixed_snake_case,
    to_snake_case,
)


def to_c_type(type_inst: GorgonzolaTypeInstance, api_name: str) -> str:
    match type_inst:
        case BaseRustTypeInfo(c_ffi_type=c_type):
            return c_type
        case GorgonzolaBaseEnum(name=name) | GorgonzolaApiObject(name=name):
            return create_c_type(name, api_name)
        case GorgonzolaConstantArray(element_type=elem) | GorgonzolaDynamicArray(element_type=elem):
            return to_c_type(elem, api_name) + '*'
        case GorgonzolaStruct(name=name):
            return f'struct {to_snake_case(name)}'
        case _:
            return 'void'


def to_c_arg(
    type_inst: GorgonzolaTypeInstance, name: str, is_const: bool, api_name: str
) -> str:
    match type_inst:
        case GorgonzolaConstantArray(element_type=elem, array_size_name=size):
            elem_c = to_c_type(elem, api_name)
            return f'const {elem_c} {name}[{size}]' if is_const else f'{elem_c} {name}[{size}]'
        case GorgonzolaDynamicArray(element_type=elem):
            elem_c = to_c_type(elem, api_name)
            return f'const {elem_c}* {name}' if is_const else f'{elem_c}* {name}'
        case BaseRustTypeInfo() | GorgonzolaBaseEnum() | GorgonzolaApiObject():
            c_type = to_c_type(type_inst, api_name)
            return f'{c_type} {name}' if is_const else f'{c_type}* {name}'
        case _:
            c_type = to_c_type(type_inst, api_name)
            return f'const {c_type}* {name}' if is_const else f'{c_type}* {name}'


def to_c_struct_field(member: GorgonzolaStructMember, api_name: str) -> str:
    if member.is_pointer:
        return f'{to_c_type(member.type, api_name)}* {member.name};'
    match member.type:
        case GorgonzolaConstantArray(element_type=elem, array_size_name=size):
            return f'{to_c_type(elem, api_name)} {member.name}[{size}];'
        case _:
            return f'{to_c_type(member.type, api_name)} {member.name};'


def to_c_padding_field(entry: LayoutEntry) -> str:
    if entry.size == U8_BYTE_SIZE:
        return f'uint8_t {entry.name};'
    elif entry.size == U16_BYTE_SIZE:
        return f'uint16_t {entry.name};'
    elif entry.size == U32_BYTE_SIZE:
        return f'uint32_t {entry.name};'
    elif entry.size == U64_BYTE_SIZE:
        return f'uint64_t {entry.name};'
    else:
        return f'uint8_t {entry.name}[{entry.size}];'


def to_c_command_def(cmd: GorgonzolaCommand, api_name: str) -> str:
    args = [
        to_c_arg(m.type, m.name, is_const=(m.direction == Direction.IN), api_name=api_name)
        for m in cmd.inputs
    ]
    prefix = f'{to_c_type(cmd.ret, api_name)} {to_snake_case(cmd.name)}('
    if not args:
        return f'{prefix}void);\n'

    indent = ' ' * len(prefix)
    lines: T.List[str] = []
    current_line = prefix

    for i, arg in enumerate(args):
        suffix = ');' if i == len(args) - 1 else ', '
        item = arg + suffix
        if len(current_line) + len(item) > MAX_LINE_LENGTH and current_line != prefix:
            lines.append(current_line.rstrip())
            current_line = indent + item
        else:
            current_line += item

    lines.append(current_line)
    return '\n'.join(lines) + '\n'


class HeaderGenerator(BaseGenerator):
    ASPECT_MASK = ApiAspectMask.FFI

    def __init__(
        self,
        full_path: Path,
        copyright_info: CopyrightInfo,
        api_name: str,
        api_data: PostProcessedApiData,
    ) -> None:
        super().__init__(full_path, copyright_info, api_name, api_data)
        self._extensible_header_emitted = False

    def _emit_header_guard_start(self) -> None:
        guard = self.full_path.name.upper().replace('.', '_')
        w = self.code_writer
        w.write_line(f'#ifndef {guard}')
        w.write_line(f'#define {guard}')
        w.write_line()
        w.write_line('#include <stdint.h>')
        w.write_line('#include <stdbool.h>')
        w.write_line('#include <stddef.h>')
        w.write_line('#include <stdalign.h>')
        w.write_line()
        w.write_line('#ifdef __cplusplus')
        w.write_line('extern "C" {')
        w.write_line('#endif')

    def _emit_header_guard_end(self) -> None:
        w = self.code_writer
        w.write_line('#ifdef __cplusplus')
        w.write_line('}')
        w.write_line('#endif')
        w.write_line()
        w.write_line('#endif')

    def emit_constant(self, obj: GorgonzolaConstant) -> None:
        self.code_writer.write_line(f'#define {obj.name} {obj.value}')

    def emit_api_object(self, obj: GorgonzolaApiObject) -> None:
        api_name = self.api_name.snake
        snake = to_prefixed_snake_case(obj.name, api_name)
        w = self.code_writer
        w.write_line()
        w.write_line(f'struct {snake};')
        w.write_line(f'typedef struct {snake}* {create_c_type(obj.name, api_name)};')

    def emit_enum(self, obj: GorgonzolaBaseEnum) -> None:
        api_name = self.api_name.snake
        prefix = to_snake_case(obj.name).upper()
        width = obj.type_info.num_bytes * 2
        w = self.code_writer
        w.write_line()
        w.write_line(f'typedef {obj.type_info.c_ffi_type} {create_c_type(obj.name, api_name)};')
        w.write_line('enum {')
        with w.indent():
            for e in obj.entries:
                entry_name = to_snake_case(e.name).upper()
                formatted_val = f'0x{e.value:0{width}X}' if e.value >= 0 else str(e.value)
                w.write_line(f'{prefix}_{entry_name} = {formatted_val},')
        w.write_line('};')

    def emit_struct(self, obj: GorgonzolaStruct, api_prefix: T.Optional[str] = None) -> None:
        api_name = self.api_name.snake
        w = self.code_writer
        w.write_line()
        w.write_line(f'{to_c_type(obj, api_name)} {{')
        with w.indent():
            if api_prefix:
                w.write_line(f'struct {api_prefix}_structure_type_header header;')
            if obj.ffi_abi_matches_protocol and obj.ffi_layout:
                members_by_name = {m.name: m for m in obj.members}
                for entry in obj.ffi_layout.entries:
                    if entry.is_padding:
                        w.write_line(to_c_padding_field(entry))
                    else:
                        member = members_by_name.get(entry.name)
                        if member:
                            w.write_line(to_c_struct_field(member, api_name))
            else:
                for member in obj.members:
                    w.write_line(to_c_struct_field(member, api_name))
        w.write_line('};')

    def _emit_extensible_header(self) -> None:
        stype_enum = self.api_data.structure_type_enum_for_aspect(self.ASPECT_MASK)
        if not stype_enum:
            return
        prefix_lower = self.api_name.snake.lower()
        prefix_upper = self.api_name.snake.upper()
        w = self.code_writer
        w.write_line()
        w.write_line('typedef enum {')
        with w.indent():
            for entry in stype_enum.entries:
                stype_name = to_snake_case(entry.name).upper()
                w.write_line(
                    f'{prefix_upper}_STRUCTURE_TYPE_{stype_name} = 0x{entry.value:08X},'
                )
        w.write_line(f'}} {prefix_lower}_structure_type_t;')
        w.write_line()
        w.write_line(f'struct {prefix_lower}_structure_type_header {{')
        with w.indent():
            w.write_line(f'{prefix_lower}_structure_type_t s_type;')
            w.write_line('uint32_t size;')
            w.write_line('alignas(8) const void* p_next;')
        w.write_line('};')

    def emit_extensible_struct(self, obj: GorgonzolaExtensibleStruct) -> None:
        if not self._extensible_header_emitted:
            self._emit_extensible_header()
            self._extensible_header_emitted = True
        self.emit_struct(obj, self.api_name.snake.lower())

    def emit_command(self, obj: GorgonzolaCommand) -> None:
        self.code_writer.write_line()
        self.code_writer.write_raw(to_c_command_def(obj, self.api_name.snake).rstrip('\n'))

    def _emit_internal(self) -> None:
        w = self.code_writer
        self.emit_license()
        w.write_line()
        self._emit_header_guard_start()

        if any(self.api_data.constants(self.ASPECT_MASK)):
            w.write_line()
        self.dispatch_all_types()
        self.dispatch_all_commands()

        w.write_line()
        self._emit_header_guard_end()
