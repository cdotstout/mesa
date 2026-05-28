# Copyright 2026 The Magma GPU Project
# SPDX-License-Identifier: MIT

from __future__ import annotations
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from enum import Enum, IntFlag
import typing as T


U8_BYTE_SIZE: int = 1
U16_BYTE_SIZE: int = 2
U32_BYTE_SIZE: int = 4
U64_BYTE_SIZE: int = 8
STRUCTURE_TYPE_HEADER_SIZE: int = 16


class OutputType(str, Enum):
    CLIENT = 'client'
    SERVER = 'server'


class Direction(str, Enum):
    IN = 'in'
    INOUT = 'inout'


class ApiAspectMask(IntFlag):
    NONE = 0
    FFI = 1
    RUST_API = 2
    IPC = 4
    VIRTIO = 8

    WIRE = IPC | VIRTIO
    COMMON = FFI | RUST_API | IPC | VIRTIO


API_ASPECT_MAP: T.Dict[str, ApiAspectMask] = {
    'none': ApiAspectMask.NONE,
    'ffi': ApiAspectMask.FFI,
    'rust_api': ApiAspectMask.RUST_API,
    'ipc': ApiAspectMask.IPC,
    'virtio': ApiAspectMask.VIRTIO,
    'wire': ApiAspectMask.WIRE,
    'common': ApiAspectMask.COMMON,
}


@dataclass
class ApiInfo:
    name: str = ''
    major_version: int = 0
    minor_version: int = 0
    patch_version: int = 0


@dataclass
class CopyrightInfo:
    spdx: str = ''
    holder: str = ''
    year: int = 0


@dataclass
class LayoutEntry:
    name: str
    type_name: str
    offset: int
    size: int
    is_padding: bool = False


@dataclass
class TypeLayout:
    size: int = 0
    alignment: int = 0
    entries: T.List[LayoutEntry] = field(default_factory=list)
    initial_offset: int = 0


class GorgonzolaTypeInstance(ABC):
    name: str
    aspect_mask: ApiAspectMask = ApiAspectMask.NONE

    @abstractmethod
    def get_dependencies(self) -> T.List[GorgonzolaTypeInstance]:
        pass


@dataclass
class BaseRustTypeInfo(GorgonzolaTypeInstance):
    name: str
    num_bytes: int
    c_ffi_type: str

    def get_dependencies(self) -> T.List[GorgonzolaTypeInstance]:
        return []


_BASE_RUST_TYPES: T.Tuple[BaseRustTypeInfo, ...] = (
    BaseRustTypeInfo('u8', U8_BYTE_SIZE, 'uint8_t'),
    BaseRustTypeInfo('i8', U8_BYTE_SIZE, 'int8_t'),
    BaseRustTypeInfo('u16', U16_BYTE_SIZE, 'uint16_t'),
    BaseRustTypeInfo('i16', U16_BYTE_SIZE, 'int16_t'),
    BaseRustTypeInfo('u32', U32_BYTE_SIZE, 'uint32_t'),
    BaseRustTypeInfo('i32', U32_BYTE_SIZE, 'int32_t'),
    BaseRustTypeInfo('u64', U64_BYTE_SIZE, 'uint64_t'),
    BaseRustTypeInfo('i64', U64_BYTE_SIZE, 'int64_t'),
    BaseRustTypeInfo('f64', U64_BYTE_SIZE, 'double'),
    BaseRustTypeInfo('usize', U64_BYTE_SIZE, 'uintptr_t'),
    BaseRustTypeInfo('c_void', U64_BYTE_SIZE, 'void*'),
)

RUST_TYPE_INFO_MAP: T.Dict[str, BaseRustTypeInfo] = {
    alias: t for t in _BASE_RUST_TYPES for alias in (t.name, t.c_ffi_type)
}


@dataclass
class GorgonzolaConstant(GorgonzolaTypeInstance):
    name: str
    type_info: BaseRustTypeInfo
    value: int

    def get_dependencies(self) -> T.List[GorgonzolaTypeInstance]:
        return []


@dataclass
class GorgonzolaEnumEntry:
    name: str
    value: int


@dataclass
class GorgonzolaBaseEnum(GorgonzolaTypeInstance):
    name: str
    type_info: BaseRustTypeInfo
    entries: T.List[GorgonzolaEnumEntry]

    def get_dependencies(self) -> T.List[GorgonzolaTypeInstance]:
        return []


class GorgonzolaEnum(GorgonzolaBaseEnum):
    pass


class GorgonzolaFlag(GorgonzolaBaseEnum):
    pass


@dataclass
class GorgonzolaApiObject(GorgonzolaTypeInstance):
    name: str

    def get_dependencies(self) -> T.List[GorgonzolaTypeInstance]:
        return []


@dataclass
class GorgonzolaConstantArray(GorgonzolaTypeInstance):
    name: str
    element_type: GorgonzolaTypeInstance
    array_size_name: str
    array_size: int
    size_constant: T.Optional[GorgonzolaConstant] = None

    def get_dependencies(self) -> T.List[GorgonzolaTypeInstance]:
        deps: T.List[GorgonzolaTypeInstance] = []
        # array: [u32; SIZE] or array: [PROP; size]
        if not isinstance(self.element_type, BaseRustTypeInfo):
            deps.append(self.element_type)
        # array: [u32; SIZE] or array: [u32; 16]
        if self.size_constant is not None:
            deps.append(self.size_constant)
        return deps


@dataclass
class GorgonzolaDynamicArray(GorgonzolaTypeInstance):
    name: str
    element_type: GorgonzolaTypeInstance
    size_member_name: str

    def get_dependencies(self) -> T.List[GorgonzolaTypeInstance]:
        if not isinstance(self.element_type, BaseRustTypeInfo):
            return [self.element_type]
        return []


@dataclass
class GorgonzolaStructMember:
    name: str
    type: GorgonzolaTypeInstance
    direction: T.Optional[Direction] = None
    is_pointer: bool = False


@dataclass
class GorgonzolaStruct(GorgonzolaTypeInstance):
    name: str
    members: T.List[GorgonzolaStructMember] = field(default_factory=list)
    protocol_layout: T.Optional[TypeLayout] = None
    ffi_layout: T.Optional[TypeLayout] = None
    ffi_abi_matches_protocol: bool = False
    has_api_object: bool = False
    has_dynamic_array: bool = False
    initial_offset: int = 0
    wire_struct: T.Optional[GorgonzolaStruct] = None

    @property
    def dynamic_array_members(self) -> T.List[GorgonzolaStructMember]:
        return [m for m in self.members if isinstance(m.type, GorgonzolaDynamicArray)]

    def get_dependencies(self) -> T.List[GorgonzolaTypeInstance]:
        deps: T.List[GorgonzolaTypeInstance] = []
        for m in self.members:
            if not isinstance(m.type, BaseRustTypeInfo):
                deps.append(m.type)
            deps.extend(m.type.get_dependencies())
        return deps


@dataclass
class GorgonzolaExtensibleStruct(GorgonzolaStruct):
    stype: str = ''
    stype_value: int = 0
    extends: T.Optional[GorgonzolaExtensibleStruct] = None
    rust_member_name: T.Optional[str] = None
    initial_offset: int = STRUCTURE_TYPE_HEADER_SIZE


class CommandKind(Enum):
    DESTRUCTOR = 'destructor'
    OBJECT_CREATION = 'object_creation'
    METHOD_CALL = 'method_call'


@dataclass
class GorgonzolaCommand:
    name: str
    ret: GorgonzolaTypeInstance
    inputs: T.List[GorgonzolaStructMember] = field(default_factory=list)
    opcode: int = 0
    target: T.Optional[str] = None
    is_destructor: bool = False
    response: bool = False
    aspect_mask: ApiAspectMask = ApiAspectMask.COMMON
    manual: bool = False
    req_struct: T.Optional[GorgonzolaStruct] = None
    resp_struct: T.Optional[GorgonzolaStruct] = None

    @property
    def kind(self) -> CommandKind:
        if self.is_destructor:
            return CommandKind.DESTRUCTOR
        if len(self.inputs_mut) == 1 and isinstance(self.inputs_mut[0].type, GorgonzolaApiObject):
            return CommandKind.OBJECT_CREATION
        return CommandKind.METHOD_CALL

    @property
    def inputs_const(self) -> T.List[GorgonzolaStructMember]:
        return [p for p in self.inputs if p.direction == Direction.IN]

    @property
    def inputs_mut(self) -> T.List[GorgonzolaStructMember]:
        return [p for p in self.inputs if p.direction == Direction.INOUT]


@dataclass
class HandleConversion:
    rust_type: str = 'u32'
    expr: str = ''
    default: str = '0'

    def resolve_expr(self, ptr_expr: str) -> str:
        if self.expr:
            return self.expr.replace('ptr', ptr_expr)
        return f'unsafe {{ (*{ptr_expr}).clone() }}'


@dataclass
class FfiConfig:
    crate_name: str = ''
    custom_imports: T.List[str] = field(default_factory=list)
    exclude_imports: T.Set[str] = field(default_factory=set)
    status_types: T.Set[str] = field(default_factory=set)
    structure_type_header: str = ''
    structure_type_enum: str = ''
    handle_conversions: T.Dict[str, HandleConversion] = field(default_factory=dict)


DEFAULT_PROTOCOL_IMPORT: str = 'use crate::protocol::*;'
DEFAULT_CODEC_IMPORTS: T.Tuple[str, ...] = (
    'use crate::util::Reader;',
    'use crate::util::Writer;',
)


@dataclass
class CodecConfig:
    protocol_import: str = DEFAULT_PROTOCOL_IMPORT
    custom_imports: T.List[str] = field(default_factory=lambda: list(DEFAULT_CODEC_IMPORTS))


@dataclass
class ApiData:
    api_name: str = ''
    constants: T.Dict[str, GorgonzolaConstant] = field(default_factory=dict)
    api_objects: T.Dict[str, GorgonzolaApiObject] = field(default_factory=dict)
    enums: T.Dict[str, GorgonzolaEnum] = field(default_factory=dict)
    flags: T.Dict[str, GorgonzolaFlag] = field(default_factory=dict)
    structs: T.Dict[str, GorgonzolaStruct] = field(default_factory=dict)
    extensible_structs: T.Dict[str, GorgonzolaExtensibleStruct] = field(default_factory=dict)
    arrays: T.Dict[str, GorgonzolaConstantArray] = field(default_factory=dict)
    dynamic_arrays: T.Dict[str, GorgonzolaDynamicArray] = field(default_factory=dict)
    commands: T.Dict[str, GorgonzolaCommand] = field(default_factory=dict)
    ffi_config: FfiConfig = field(default_factory=FfiConfig)
    encoder_config: CodecConfig = field(default_factory=CodecConfig)
    decoder_config: CodecConfig = field(default_factory=CodecConfig)
    _types_by_name: T.Dict[str, GorgonzolaTypeInstance] = field(default_factory=dict)

    def add_type(self, obj: GorgonzolaTypeInstance) -> None:
        self._types_by_name[obj.name] = obj
        match obj:
            case GorgonzolaConstant():
                self.constants[obj.name] = obj
            case GorgonzolaApiObject():
                self.api_objects[obj.name] = obj
            case GorgonzolaEnum():
                self.enums[obj.name] = obj
            case GorgonzolaFlag():
                self.flags[obj.name] = obj
            case GorgonzolaExtensibleStruct():
                self.extensible_structs[obj.name] = obj
            case GorgonzolaStruct():
                self.structs[obj.name] = obj
            case GorgonzolaConstantArray():
                self.arrays[obj.name] = obj
            case GorgonzolaDynamicArray():
                self.dynamic_arrays[obj.name] = obj

    def add_command(self, cmd: GorgonzolaCommand) -> None:
        self.commands[cmd.name] = cmd

    def all_types(self) -> T.Iterator[GorgonzolaTypeInstance]:
        yield from self.constants.values()
        yield from self.api_objects.values()
        yield from self.enums.values()
        yield from self.flags.values()
        yield from self.structs.values()
        yield from self.extensible_structs.values()
        yield from self.arrays.values()
        yield from self.dynamic_arrays.values()

    def find_by_name(self, name: str) -> T.Optional[GorgonzolaTypeInstance]:
        if name == 'void':
            return BaseRustTypeInfo('void', 0, 'void')
        return RUST_TYPE_INFO_MAP.get(name) or self._types_by_name.get(name)

    def resolve_type_name(self, name: str) -> GorgonzolaTypeInstance:
        obj = self.find_by_name(name)
        if obj is not None:
            return obj
        raise ValueError(f'Unable to resolve type name: {name}')


def to_rust_protocol_type(type_inst: GorgonzolaTypeInstance) -> str:
    match type_inst:
        case BaseRustTypeInfo(name='void'):
            return '()'
        case BaseRustTypeInfo(name=name):
            return name
        case GorgonzolaApiObject():
            return 'u32'
        case GorgonzolaConstantArray(element_type=elem, array_size_name=size):
            elem_str = to_rust_protocol_type(elem)
            return f'[{elem_str}; {size}]'
        case GorgonzolaDynamicArray(element_type=elem):
            elem_str = to_rust_protocol_type(elem)
            return f'Vec<{elem_str}>'
        case GorgonzolaBaseEnum(name=name) | GorgonzolaStruct(name=name):
            return name
        case _:
            return type_inst.name
