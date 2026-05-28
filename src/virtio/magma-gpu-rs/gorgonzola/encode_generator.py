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


class EncodeGenerator(CommonGenerator):
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
        cmd_pascal = normalize_command_name(obj.name, api)
        cmd_snake = to_snake_case(cmd_pascal)

        members = obj.req_struct.members if obj.req_struct else obj.inputs_const
        ext_members = [
            m for m in members
            if isinstance(m.type, GorgonzolaExtensibleStruct)
        ]

        self.code_writer.set_indent_multiple(1)
        self.code_writer.write_fn(
            f'encode_{cmd_snake}',
            ['&mut self', f'msg: &{cmd_pascal}'],
            ret='std::io::Result<()>',
        )
        self.code_writer.set_indent_multiple(2)
        if ext_members:
            self.code_writer.write_line('let mut cmd = *msg;')
            for m in ext_members:
                self.code_writer.write_line(f'let p_next_{m.name} = cmd.{m.name}.header.p_next;')
                self.code_writer.write_line(f'cmd.{m.name}.header.p_next = if p_next_{m.name} != 0 {{ 1 }} else {{ 0 }};')
            self.code_writer.write_line('self.write_bytes(zerocopy::IntoBytes::as_bytes(&cmd))?;')
            for m in ext_members:
                self.code_writer.write_line(f'if p_next_{m.name} != 0 {{')
                self.code_writer.set_indent_multiple(3)
                self.code_writer.write_line(f'self.encode_extensible_chain(p_next_{m.name})?;')
                self.code_writer.set_indent_multiple(2)
                self.code_writer.write_line('}')
            self.code_writer.write_line('Ok(())')
        else:
            self.code_writer.write_line('self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))')
        self.code_writer.set_indent_multiple(1)
        self.code_writer.write_line('}')

        self.req_arms.append(
            (f'{api}Protocol::{cmd_pascal}(msg)', f'self.encode_{cmd_snake}(msg)')
        )

        if obj.response:
            resp_snake = f'resp_{cmd_snake}'
            resp_variant = f'{cmd_pascal}Resp'
            self.code_writer.set_indent_multiple(1)
            self.code_writer.write_fn(
                f'encode_{resp_snake}',
                ['&mut self', f'msg: &{resp_variant}'],
                ret='std::io::Result<()>',
            )
            self.code_writer.set_indent_multiple(2)
            self.code_writer.write_line('self.write_bytes(zerocopy::IntoBytes::as_bytes(msg))')
            self.code_writer.set_indent_multiple(1)
            self.code_writer.write_line('}')

            self.resp_arms.append(
                (f'{api}Protocol::{resp_variant}(msg)', f'self.encode_{resp_snake}(msg)')
            )

    def emit_extensible_struct(self, obj: GorgonzolaExtensibleStruct) -> None:
        api = self.api_name.pascal
        variant = normalize_command_name(obj.name, api)
        struct_snake = to_snake_case(variant)

        self.code_writer.set_indent_multiple(1)
        self.code_writer.write_fn(
            f'encode_{struct_snake}',
            ['&mut self', f'msg: &{obj.name}'],
            ret='std::io::Result<()>',
        )
        self.code_writer.set_indent_multiple(2)
        self.code_writer.write_line('let mut obj = *msg;')
        self.code_writer.write_line('let p_next = obj.header.p_next;')
        self.code_writer.write_line('obj.header.p_next = if p_next != 0 { 1 } else { 0 };')
        self.code_writer.write_line('self.write_bytes(zerocopy::IntoBytes::as_bytes(&obj))?;')
        self.code_writer.write_line('if p_next != 0 {')
        self.code_writer.set_indent_multiple(3)
        self.code_writer.write_line('self.encode_extensible_chain(p_next)?;')
        self.code_writer.set_indent_multiple(2)
        self.code_writer.write_line('}')
        self.code_writer.write_line('Ok(())')
        self.code_writer.set_indent_multiple(1)
        self.code_writer.write_line('}')

        self.ext_arms.append(
            (f'{api}ExtensibleStruct::{variant}(msg)', f'self.encode_{struct_snake}(msg)')
        )

    def _emit_extensible_chain(self) -> None:
        api = self.api_name.pascal
        api_upper = api.upper()
        self.code_writer.set_indent_multiple(1)
        self.code_writer.write_fn(
            'encode_extensible_chain',
            ['&mut self', 'mut curr: u64'],
            ret='std::io::Result<()>',
        )
        self.code_writer.set_indent_multiple(2)
        self.code_writer.write_line('while curr != 0 {')
        self.code_writer.set_indent_multiple(3)
        self.code_writer.write_line(f'let header = unsafe {{ &*(curr as *const {api}StructureTypeHeader) }};')
        self.code_writer.write_line('let next = header.p_next;')
        self.code_writer.write_line('match header.stype {')
        self.code_writer.set_indent_multiple(4)
        for ext in self.api_data.extensible_structs_for_aspect(self.ASPECT_MASK):
            stype_const = f'{api_upper}_STRUCTURE_TYPE_{to_snake_case(ext.stype).upper()}'
            self.code_writer.write_line(f'{stype_const} => {{')
            self.code_writer.set_indent_multiple(5)
            self.code_writer.write_line(f'let mut ext = unsafe {{ *(curr as *const {ext.name}) }};')
            self.code_writer.write_line('ext.header.p_next = if next != 0 { 1 } else { 0 };')
            self.code_writer.write_line('self.write_bytes(zerocopy::IntoBytes::as_bytes(&ext))?;')
            self.code_writer.set_indent_multiple(4)
            self.code_writer.write_line('}')
        self.code_writer.write_line('_ => {')
        self.code_writer.set_indent_multiple(5)
        self.code_writer.write_line('return Err(std::io::Error::new(')
        self.code_writer.set_indent_multiple(6)
        self.code_writer.write_line('std::io::ErrorKind::InvalidData,')
        self.code_writer.write_line('"unknown stype",')
        self.code_writer.set_indent_multiple(5)
        self.code_writer.write_line('))')
        self.code_writer.set_indent_multiple(4)
        self.code_writer.write_line('}')
        self.code_writer.set_indent_multiple(3)
        self.code_writer.write_line('}')
        self.code_writer.write_line('curr = next;')
        self.code_writer.set_indent_multiple(2)
        self.code_writer.write_line('}')
        self.code_writer.write_line('Ok(())')
        self.code_writer.set_indent_multiple(1)
        self.code_writer.write_line('}')

    def _emit_imports(self) -> str:
        encoder_config = self.api_data.encoder_config
        protocol_import = encoder_config.get('protocol_import', 'use crate::protocol::*;')
        custom_imports = encoder_config.get(
            'custom_imports', ['use crate::util::Reader;', 'use crate::util::Writer;']
        )
        imports = f'{protocol_import}\n'
        for imp in custom_imports:
            imports += f'{imp}\n'
        return imports

    def _emit_internal(self) -> None:
        self.code_writer.write_raw(self._license().rstrip('\n'))
        self.code_writer.write_raw(self._emit_imports().rstrip('\n'))
        self.code_writer.write_line('pub struct Encoder<W> {')
        self.code_writer.set_indent_multiple(1)
        self.code_writer.write_line('writer: W,')
        self.code_writer.write_line('bytes_written: usize,')
        self.code_writer.set_indent_multiple(0)
        self.code_writer.write_line('}')
        self.code_writer.write_line()
        self.code_writer.write_line('impl<W: std::io::Write> Encoder<W> {')
        self.code_writer.set_indent_multiple(1)
        self.code_writer.write_line('pub fn new(writer: W) -> Encoder<W> {')
        self.code_writer.set_indent_multiple(2)
        self.code_writer.write_line('Encoder {')
        self.code_writer.set_indent_multiple(3)
        self.code_writer.write_line('writer,')
        self.code_writer.write_line('bytes_written: 0,')
        self.code_writer.set_indent_multiple(2)
        self.code_writer.write_line('}')
        self.code_writer.set_indent_multiple(1)
        self.code_writer.write_line('}')
        self.code_writer.write_line()
        self.code_writer.write_line('pub fn bytes_written(&self) -> usize {')
        self.code_writer.set_indent_multiple(2)
        self.code_writer.write_line('self.bytes_written')
        self.code_writer.set_indent_multiple(1)
        self.code_writer.write_line('}')
        self.code_writer.write_line()
        self.code_writer.write_line('pub fn into_inner(self) -> W {')
        self.code_writer.set_indent_multiple(2)
        self.code_writer.write_line('self.writer')
        self.code_writer.set_indent_multiple(1)
        self.code_writer.write_line('}')
        self.code_writer.write_line()
        self.code_writer.write_line('pub fn writer_mut(&mut self) -> &mut W {')
        self.code_writer.set_indent_multiple(2)
        self.code_writer.write_line('&mut self.writer')
        self.code_writer.set_indent_multiple(1)
        self.code_writer.write_line('}')
        self.code_writer.write_line()
        self.code_writer.write_line('pub fn writer(&self) -> &W {')
        self.code_writer.set_indent_multiple(2)
        self.code_writer.write_line('&self.writer')
        self.code_writer.set_indent_multiple(1)
        self.code_writer.write_line('}')
        self.code_writer.write_line()
        self.code_writer.write_line('fn write_bytes(&mut self, bytes: &[u8]) -> std::io::Result<()> {')
        self.code_writer.set_indent_multiple(2)
        self.code_writer.write_line('self.writer.write_all(bytes)?;')
        self.code_writer.write_line('self.bytes_written += bytes.len();')
        self.code_writer.write_line('Ok(())')
        self.code_writer.set_indent_multiple(1)
        self.code_writer.write_line('}')

        if self.api_data.extensible_structs:
            self._emit_extensible_chain()

        for t in self.api_data.types(self.ASPECT_MASK):
            self.dispatch_type(t)
        for cmd in self.api_data.commands_for_aspect(self.ASPECT_MASK):
            self.dispatch_command(cmd)

        api = self.api_name.pascal
        self.code_writer.set_indent_multiple(1)
        self.code_writer.write_fn(
            'encode_msg',
            ['&mut self', f'msg: &{api}Protocol'],
            ret='std::io::Result<()>',
        )
        self.code_writer.set_indent_multiple(2)
        self.code_writer.write_line('match msg {')
        self.code_writer.set_indent_multiple(3)
        for pat, expr in self.req_arms:
            self.code_writer.write_match_arm(pat, expr)
        if self.resp_arms:
            for pat, expr in self.resp_arms:
                self.code_writer.write_match_arm(pat, expr)
        self.code_writer.set_indent_multiple(2)
        self.code_writer.write_line('}')
        self.code_writer.set_indent_multiple(1)
        self.code_writer.write_line('}')

        if self.ext_arms:
            self.code_writer.write_line()
            self.code_writer.write_line('#[allow(dead_code)]')
            self.code_writer.write_fn(
                'encode_extensible_struct',
                ['&mut self', f'msg: &{api}ExtensibleStruct'],
                ret='std::io::Result<()>',
            )
            self.code_writer.set_indent_multiple(2)
            self.code_writer.write_line('match msg {')
            self.code_writer.set_indent_multiple(3)
            for pat, expr in self.ext_arms:
                self.code_writer.write_match_arm(pat, expr)
            self.code_writer.set_indent_multiple(2)
            self.code_writer.write_line('}')
            self.code_writer.set_indent_multiple(1)
            self.code_writer.write_line('}')

        self.code_writer.set_indent_multiple(0)
        self.code_writer.write_line('}')
        self.code_writer.write_line()
        self.code_writer.write_line('impl<\'a> Encoder<&\'a mut [u8]> {')
        self.code_writer.set_indent_multiple(1)
        self.code_writer.write_line('pub fn from_slice(buf: &\'a mut [u8]) -> Encoder<&\'a mut [u8]> {')
        self.code_writer.set_indent_multiple(2)
        self.code_writer.write_line('Encoder::new(buf)')
        self.code_writer.set_indent_multiple(1)
        self.code_writer.write_line('}')
        self.code_writer.set_indent_multiple(0)
        self.code_writer.write_line('}')

