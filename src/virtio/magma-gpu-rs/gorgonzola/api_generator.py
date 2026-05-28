# Copyright 2026 The Magma GPU Project
# SPDX-License-Identifier: MIT

from __future__ import annotations
from pathlib import Path
import tomllib
import typing as T

from gorgonzola.base_generator import BaseGenerator
from gorgonzola.common import (
    API_ASPECT_MAP,
    ApiAspectMask,
    ApiData,
    ApiInfo,
    CodecConfig,
    CopyrightInfo,
    Direction,
    FfiConfig,
    GorgonzolaApiObject,
    GorgonzolaCommand,
    GorgonzolaConstant,
    GorgonzolaConstantArray,
    GorgonzolaDynamicArray,
    GorgonzolaEnum,
    GorgonzolaEnumEntry,
    GorgonzolaExtensibleStruct,
    GorgonzolaFlag,
    GorgonzolaStruct,
    GorgonzolaStructMember,
    GorgonzolaTypeInstance,
    HandleConversion,
    OutputType,
    RUST_TYPE_INFO_MAP,
)
from gorgonzola.decode_generator import DecodeGenerator
from gorgonzola.encode_generator import EncodeGenerator
from gorgonzola.ffi_generator import FFIGenerator
from gorgonzola.header_generator import HeaderGenerator
from gorgonzola.post_process import PostProcessedApiData
from gorgonzola.protocol_generator import ProtocolGenerator
from gorgonzola.rust_api_generator import RustApiGenerator
from gorgonzola.utils import to_pascal_case, to_snake_case

OPCODE_START: int = 1


class ApiSection(T.TypedDict, total=False):
    name: str
    major_version: int
    minor_version: int
    patch_version: int


class CopyrightSection(T.TypedDict, total=False):
    spdx: str
    holder: str
    year: int


class ClientOutDir(T.TypedDict, total=False):
    ffi: str
    header: str
    protocol: str
    rust_api: str
    encoder: str
    decoder: str


class ServerOutDir(T.TypedDict, total=False):
    protocol: str
    rust_api: str
    decoder: str


class OutDirInfoSection(T.TypedDict, total=False):
    client: ClientOutDir
    server: ServerOutDir


class HandleConversionSection(T.TypedDict, total=False):
    rust_type: str
    expr: str
    default: str


class FfiSection(T.TypedDict, total=False):
    crate_name: str
    structure_type_header: str
    structure_type_enum: str
    custom_imports: T.List[str]
    exclude_imports: T.List[str]
    status_types: T.List[str]
    handle_conversions: T.Dict[str, HandleConversionSection]


class CodecSection(T.TypedDict, total=False):
    protocol_import: str
    custom_imports: T.List[str]


class ApiToml(T.TypedDict, total=False):
    api: ApiSection
    copyright: CopyrightSection
    out_dir_info: OutDirInfoSection
    ffi: FfiSection
    encoder: CodecSection
    decoder: CodecSection


class Constant(T.TypedDict, total=False):
    name: str
    type: str
    value: int
    aspects: T.List[str]


class Enum(T.TypedDict, total=False):
    name: str
    type: str
    entries: T.Dict[str, int]
    aspects: T.List[str]


class DefinitionsToml(T.TypedDict, total=False):
    api_objects: T.List[str]
    constants: T.List[Constant]
    enums: T.List[Enum]
    flags: T.List[Enum]


class Array(T.TypedDict, total=False):
    size: str


class MemberSpec(T.TypedDict, total=False):
    name: str
    type: str
    array: Array


class StructMember(MemberSpec, total=False):
    pointer: bool


class Stype(T.TypedDict, total=False):
    name: str
    value: int


class Struct(T.TypedDict, total=False):
    name: str
    members: T.List[StructMember]
    aspects: T.List[str]


class ExtensibleStruct(Struct, total=False):
    stype: Stype
    extends: str
    rust_member_name: str


class StructsToml(T.TypedDict, total=False):
    structs: T.List[Struct]
    extensible_structs: T.List[ExtensibleStruct]


class CommandInput(MemberSpec, total=False):
    direction: str


class Command(T.TypedDict, total=False):
    name: str
    target: str
    inputs: T.List[CommandInput]
    output: str
    aspects: T.List[str]
    destructor: bool
    response: bool
    manual: bool


class CommandsToml(T.TypedDict, total=False):
    commands: T.List[Command]


_T = T.TypeVar('_T')


def load_toml(file: Path, type_hint: T.Type[_T]) -> _T:
    with file.open(mode='rb') as f:
        return T.cast(_T, tomllib.load(f))


class ApiGenerator:
    def __init__(self, directory: Path, out_dir: Path, output_type: OutputType) -> None:
        self.directory = directory
        self.out_dir = out_dir
        self.output_type = output_type
        self.generators: T.List[BaseGenerator] = []
        self.common_api_data = ApiData()
        self.api_info = ApiInfo()
        self.copyright_info = CopyrightInfo()

    def _parse_aspect_mask(
        self, item: T.Mapping[str, object], default: ApiAspectMask = ApiAspectMask.COMMON
    ) -> ApiAspectMask:
        aspects = item.get('aspects')
        if not isinstance(aspects, list):
            return default
        mask = ApiAspectMask.NONE
        for aspect in aspects:
            mask |= API_ASPECT_MAP[str(aspect)]
        return mask

    def _add_type(
        self, item: GorgonzolaTypeInstance, aspect: ApiAspectMask = ApiAspectMask.NONE
    ) -> None:
        item.aspect_mask = aspect
        self.common_api_data.add_type(item)

    def _add_command(
        self, item: GorgonzolaCommand, aspect: ApiAspectMask = ApiAspectMask.COMMON
    ) -> None:
        item.aspect_mask = aspect
        self.common_api_data.add_command(item)

    def parse(self) -> None:
        output_data = self._parse_api_toml()
        self._parse_definitions_toml()
        self._parse_structs_toml()
        self._parse_commands_toml()

        post_processed_api_data = PostProcessedApiData(self.common_api_data)
        self._configure_generators(output_data, post_processed_api_data)

    def emit(self) -> None:
        for generator in self.generators:
            generator.emit()

    def _parse_api_toml(self) -> OutDirInfoSection:
        api_toml_path = self.directory / 'api.toml'
        api_toml = load_toml(api_toml_path, ApiToml)

        api_data = api_toml['api']
        copyright_data = api_toml['copyright']

        self.api_info = ApiInfo(
            name=api_data['name'],
            major_version=api_data['major_version'],
            minor_version=api_data['minor_version'],
            patch_version=api_data['patch_version'],
        )

        self.copyright_info = CopyrightInfo(
            spdx=copyright_data['spdx'],
            holder=copyright_data['holder'],
            year=copyright_data['year'],
        )
        api_name = self.api_info.name
        api_pascal = to_pascal_case(api_name)
        api_snake = to_snake_case(api_name).lower()

        self.common_api_data.api_name = api_name

        ffi_raw = api_toml.get('ffi', {})
        handle_convs = {
            k: HandleConversion(
                rust_type=v.get('rust_type', 'u32'),
                expr=v.get('expr', ''),
                default=v.get('default', '0'),
            )
            for k, v in ffi_raw.get('handle_conversions', {}).items()
        }

        self.common_api_data.ffi_config = FfiConfig(
            crate_name=ffi_raw.get('crate_name', api_snake),
            structure_type_header=ffi_raw.get(
                'structure_type_header', f'{api_pascal}StructureTypeHeader'
            ),
            structure_type_enum=ffi_raw.get(
                'structure_type_enum', f'{api_pascal}StructureType'
            ),
            custom_imports=list(ffi_raw.get('custom_imports', [])),
            exclude_imports=set(ffi_raw.get('exclude_imports', [])),
            status_types=set(ffi_raw.get('status_types', [])),
            handle_conversions=handle_convs,
        )

        default_codec_imports = ['use crate::util::Reader;', 'use crate::util::Writer;']
        enc_raw = api_toml.get('encoder', {})
        self.common_api_data.encoder_config = CodecConfig(
            protocol_import=enc_raw.get('protocol_import', 'use crate::protocol::*;'),
            custom_imports=list(enc_raw.get('custom_imports', default_codec_imports)),
        )

        dec_raw = api_toml.get('decoder') or enc_raw
        self.common_api_data.decoder_config = CodecConfig(
            protocol_import=dec_raw.get('protocol_import', 'use crate::protocol::*;'),
            custom_imports=list(dec_raw.get('custom_imports', default_codec_imports)),
        )

        return api_toml.get('out_dir_info', {})

    def _configure_generators(
        self, output_data: OutDirInfoSection, post_processed_api_data: PostProcessedApiData
    ) -> None:
        copyright_info = self.copyright_info
        api_name = self.api_info.name

        def add_gen(cls: T.Type[BaseGenerator], rel_path: T.Optional[str]) -> None:
            if rel_path:
                self.generators.append(
                    cls(
                        self.out_dir / Path(rel_path),
                        copyright_info,
                        api_name,
                        post_processed_api_data,
                    )
                )

        if self.output_type == OutputType.CLIENT:
            client = output_data.get('client', {})
            add_gen(FFIGenerator, client.get('ffi'))
            add_gen(HeaderGenerator, client.get('header'))
            add_gen(ProtocolGenerator, client.get('protocol'))
            add_gen(RustApiGenerator, client.get('rust_api'))
            add_gen(EncodeGenerator, client.get('encoder'))
            add_gen(DecodeGenerator, client.get('decoder'))
        elif self.output_type == OutputType.SERVER:
            server = output_data.get('server', {})
            add_gen(ProtocolGenerator, server.get('protocol'))
            add_gen(RustApiGenerator, server.get('rust_api'))
            add_gen(DecodeGenerator, server.get('decoder'))

    def _parse_base_enum(
        self,
        cls: T.Type[GorgonzolaEnum] | T.Type[GorgonzolaFlag],
        item: Enum,
        kind: str,
    ) -> None:
        name = item['name']
        entries = [GorgonzolaEnumEntry(name=k, value=v) for k, v in item['entries'].items()]
        type_info = RUST_TYPE_INFO_MAP.get(item['type'])
        if not type_info:
            raise ValueError(f'Unknown underlying type {item["type"]} for {kind} {name}')
        mask = self._parse_aspect_mask(item)
        self._add_type(cls(name, type_info, entries), mask)

    def _parse_definitions_toml(self) -> None:
        definitions_toml = load_toml(self.directory / 'definitions.toml', DefinitionsToml)

        for obj_name in definitions_toml.get('api_objects', []):
            self._add_type(GorgonzolaApiObject(obj_name), ApiAspectMask.COMMON)

        for c in definitions_toml.get('constants', []):
            mask = self._parse_aspect_mask(c)
            type_info = RUST_TYPE_INFO_MAP.get(c['type'])
            if not type_info:
                raise ValueError(
                    f'Unknown underlying type {c["type"]} for constant {c["name"]}'
                )
            self._add_type(GorgonzolaConstant(c['name'], type_info, c['value']), mask)

        for e in definitions_toml.get('enums', []):
            self._parse_base_enum(GorgonzolaEnum, e, 'enum')

        for f in definitions_toml.get('flags', []):
            self._parse_base_enum(GorgonzolaFlag, f, 'flag')

    def _register_struct_declaration(self, struct_data: Struct) -> None:
        name = struct_data['name']
        aspect_mask = self._parse_aspect_mask(struct_data, default=ApiAspectMask.NONE)
        struct_obj = GorgonzolaStruct(name=name, members=[])
        self._add_type(struct_obj, aspect_mask)

    def _register_extensible_struct_declaration(self, struct_data: ExtensibleStruct) -> None:
        name = struct_data['name']
        aspect_mask = self._parse_aspect_mask(struct_data, default=ApiAspectMask.NONE)
        stype_data = struct_data['stype']
        stype = stype_data.get('name', '')
        stype_value = stype_data.get('value', 0)
        rust_member_name = struct_data.get('rust_member_name')
        struct_obj = GorgonzolaExtensibleStruct(
            name=name,
            members=[],
            stype=stype,
            stype_value=stype_value,
            extends=None,
            rust_member_name=rust_member_name,
        )
        self._add_type(struct_obj, aspect_mask)

    def _resolve_member_type(
        self,
        member: MemberSpec,
        sibling_names: T.Optional[T.Set[str]] = None,
    ) -> GorgonzolaTypeInstance:
        type_name = member['type']
        if 'array' in member:
            size_str = str(member['array'].get('size', ''))
            if size_str:
                resolved_elem_type = self.common_api_data.resolve_type_name(type_name)
                if size_str.isdigit() or size_str in self.common_api_data.constants:
                    arr_name = f'[{type_name}; {size_str}]'
                    if arr_name not in self.common_api_data.arrays:
                        if size_str.isdigit():
                            array_size = int(size_str)
                            size_constant: T.Optional[GorgonzolaConstant] = None
                        else:
                            size_constant = self.common_api_data.constants[size_str]
                            array_size = size_constant.value
                        array_type = GorgonzolaConstantArray(
                            arr_name,
                            resolved_elem_type,
                            size_str,
                            array_size,
                            size_constant,
                        )
                        self._add_type(array_type)
                    type_name = arr_name
                elif sibling_names is not None and size_str in sibling_names:
                    arr_name = f'Vec<{type_name};{size_str}>'
                    if arr_name not in self.common_api_data.dynamic_arrays:
                        dyn_type = GorgonzolaDynamicArray(
                            arr_name,
                            resolved_elem_type,
                            size_str,
                        )
                        self._add_type(dyn_type)
                    type_name = arr_name
                else:
                    raise ValueError(f'Unknown array size constant: {size_str}')
        return self.common_api_data.resolve_type_name(type_name)

    def _resolve_struct_members(
        self,
        struct_data: Struct,
        struct_dict: T.Mapping[str, GorgonzolaStruct],
    ) -> None:
        name = struct_data['name']
        struct_obj = struct_dict[name]
        raw_members = struct_data.get('members', [])
        sibling_names = {m['name'] for m in raw_members}
        for member in raw_members:
            resolved_type = self._resolve_member_type(member, sibling_names)
            is_pointer = member.get('pointer', False)
            struct_obj.members.append(
                GorgonzolaStructMember(
                    name=member['name'],
                    type=resolved_type,
                    is_pointer=is_pointer,
                )
            )

    def _parse_structs_toml(self) -> None:
        data = load_toml(self.directory / 'structs.toml', StructsToml)

        structs_list = data.get('structs', [])
        ext_structs_list = data.get('extensible_structs', [])

        for s in structs_list:
            self._register_struct_declaration(s)

        for s in ext_structs_list:
            self._register_extensible_struct_declaration(s)

        for s in structs_list:
            self._resolve_struct_members(s, self.common_api_data.structs)

        for s in ext_structs_list:
            self._resolve_struct_members(s, self.common_api_data.extensible_structs)

        # Validate extensible structs
        stype_values: T.Set[int] = set()
        for s in ext_structs_list:
            name = s['name']
            struct_obj = self.common_api_data.extensible_structs[name]
            if struct_obj.stype_value in stype_values:
                raise ValueError(
                    f"Duplicate stype_value '{struct_obj.stype_value}' in extensible struct '{name}'."
                )
            stype_values.add(struct_obj.stype_value)

            extends_name = s.get('extends')
            if extends_name:
                if extends_name not in self.common_api_data.extensible_structs:
                    raise ValueError(
                        f"Extensible struct '{name}' specifies extends='{extends_name}', "
                        f'which is not a defined extensible struct.'
                    )
                struct_obj.extends = self.common_api_data.extensible_structs[extends_name]

    def _parse_commands_toml(self) -> None:
        data = load_toml(self.directory / 'commands.toml', CommandsToml)
        next_opcode = OPCODE_START
        for command in data.get('commands', []):
            name = command['name']
            aspect_mask = self._parse_aspect_mask(command, default=ApiAspectMask.COMMON)
            manual = command.get('manual', False)

            inputs = self._parse_fields(command.get('inputs', []))

            ret_name = command.get('output') or 'void'
            ret = self.common_api_data.resolve_type_name(ret_name)
            target = command.get('target')
            is_destructor = command.get('destructor', False)
            response = command.get('response', False)

            opcode_val = 0
            if aspect_mask & ApiAspectMask.WIRE:
                opcode_val = next_opcode
                next_opcode += 1

            self._add_command(
                GorgonzolaCommand(
                    name=name,
                    ret=ret,
                    inputs=inputs,
                    opcode=opcode_val,
                    target=target,
                    is_destructor=is_destructor,
                    response=response,
                    manual=manual,
                ),
                aspect_mask,
            )

    def _parse_fields(self, fields_data: T.List[CommandInput]) -> T.List[GorgonzolaStructMember]:
        fields = []
        sibling_names = {f['name'] for f in fields_data}
        for f in fields_data:
            resolved_type = self._resolve_member_type(f, sibling_names)
            direction_str = f.get('direction')
            if direction_str not in (Direction.IN, Direction.INOUT):
                raise ValueError(
                    f'Invalid direction "{direction_str}" for input "{f.get("name")}". '
                    'Must be "in" or "inout".'
                )
            direction = Direction(direction_str)
            fields.append(
                GorgonzolaStructMember(name=f['name'], type=resolved_type, direction=direction)
            )
        return fields
