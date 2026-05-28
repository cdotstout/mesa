# Copyright 2026 The Magma GPU Project
# SPDX-License-Identifier: MIT

from __future__ import annotations

from pathlib import Path
import typing as T

from gorgonzola.common import (
    CommonGenerator,
    ApiAspectMask,
    CopyrightInfo,
    GorgonzolaExtensibleStruct,
    GorgonzolaCommand,
)
from gorgonzola.post_process import PostProcessedApiData
from gorgonzola.utils import normalize_command_name, to_snake_case


class DecodeGenerator(CommonGenerator):
    ASPECT_MASK = ApiAspectMask.WIRE

    def __init__(
        self,
        full_path: Path,
        copyright_info: CopyrightInfo,
        api_name: str,
        api_data: PostProcessedApiData,
    ) -> None:
        super().__init__(full_path, copyright_info, api_name, api_data)
        self.req_arms: T.List[T.Tuple[str, str]] = []
        self.resp_arms: T.List[T.Tuple[str, str]] = []
        self.ext_arms: T.List[T.Tuple[str, str]] = []

    def emit_command(self, obj: GorgonzolaCommand) -> None:
        api = self.api_name.pascal
        api_upper = api.upper()
        cmd_pascal = normalize_command_name(obj.name, api)
        cmd_snake = to_snake_case(cmd_pascal).upper()
        opcode_const = f'{api_upper}_OPCODE_{cmd_snake}'
        self.req_arms.append((opcode_const, f'{api}Protocol::{cmd_pascal}'))

        if obj.response:
            resp_opcode_const = f'{api_upper}_OPCODE_RESP_{cmd_snake}'
            resp_name = f'{cmd_pascal}Resp'
            self.resp_arms.append((resp_opcode_const, f'{api}Protocol::{resp_name}'))

    def emit_extensible_struct(self, obj: GorgonzolaExtensibleStruct) -> None:
        api = self.api_name.pascal
        api_upper = api.upper()
        stype_name = to_snake_case(obj.stype).upper()
        stype_const = f'{api_upper}_STRUCTURE_TYPE_{stype_name}'
        struct_pascal = normalize_command_name(obj.name, api)
        self.ext_arms.append((stype_const, f'{api}ExtensibleStruct::{struct_pascal}'))

    def _emit_imports(self) -> str:
        decoder_config = self.api_data.decoder_config or self.api_data.encoder_config
        protocol_import = decoder_config.get('protocol_import', 'use crate::protocol::*;')
        custom_imports = decoder_config.get(
            'custom_imports', ['use crate::util::Reader;', 'use crate::util::Writer;']
        )
        imports = f'{protocol_import}\n'
        for imp in custom_imports:
            imports += f'{imp}\n'
        return imports

    def _emit_internal(self) -> None:
        for t in self.api_data.types(self.ASPECT_MASK):
            self.dispatch_type(t)
        for cmd in self.api_data.commands_for_aspect(self.ASPECT_MASK):
            self.dispatch_command(cmd)

        api = self.api_name.pascal
        self.code_writer.write_raw(self._license().rstrip('\n'))
        self.code_writer.write_raw(self._emit_imports().rstrip('\n'))
        self.code_writer.write_line()
        self.code_writer.write_fn('decode', ['reader: &mut Reader'], ret=f'Option<{api}Protocol>')
        self.code_writer.set_indent_multiple(1)
        self.code_writer.write_line(
            f'if reader.available_bytes() < std::mem::size_of::<{api}CommandHeader>() {{'
        )
        self.code_writer.set_indent_multiple(2)
        self.code_writer.write_line('return None;')
        self.code_writer.set_indent_multiple(1)
        self.code_writer.write_line('}')
        self.code_writer.write_line(f'let header = reader.peek_obj::<{api}CommandHeader>().ok()?;')
        self.code_writer.write_line('match header.opcode {')
        self.code_writer.set_indent_multiple(2)
        for pat, var in self.req_arms:
            self.code_writer.write_decoder_match_arm(pat, var)
        if self.resp_arms:
            self.code_writer.write_line()
            for pat, var in self.resp_arms:
                self.code_writer.write_decoder_match_arm(pat, var)
        self.code_writer.write_line('_ => None,')
        self.code_writer.set_indent_multiple(1)
        self.code_writer.write_line('}')
        self.code_writer.set_indent_multiple(0)
        self.code_writer.write_line('}')

        if self.ext_arms:
            self.code_writer.write_line()
            self.code_writer.write_line('#[allow(dead_code)]')
            self.code_writer.write_fn(
                'decode_extensible_struct',
                ['reader: &mut Reader'],
                ret=f'Option<{api}ExtensibleStruct>',
            )
            self.code_writer.set_indent_multiple(1)
            self.code_writer.write_line(
                f'if reader.available_bytes() < std::mem::size_of::<{api}StructureTypeHeader>() {{'
            )
            self.code_writer.set_indent_multiple(2)
            self.code_writer.write_line('return None;')
            self.code_writer.set_indent_multiple(1)
            self.code_writer.write_line('}')
            self.code_writer.write_line(
                f'let header = reader.peek_obj::<{api}StructureTypeHeader>().ok()?;'
            )
            self.code_writer.write_line('match header.stype {')
            self.code_writer.set_indent_multiple(2)
            for pat, var in self.ext_arms:
                self.code_writer.write_decoder_match_arm(pat, var)
            self.code_writer.write_line('_ => None,')
            self.code_writer.set_indent_multiple(1)
            self.code_writer.write_line('}')
            self.code_writer.set_indent_multiple(0)
            self.code_writer.write_line('}')

