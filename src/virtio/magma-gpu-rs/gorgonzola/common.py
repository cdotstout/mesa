# Copyright 2026 The Magma GPU Project
# SPDX-License-Identifier: MIT

from __future__ import annotations
from abc import ABC, abstractmethod
import os
from pathlib import Path
import typing as T
from dataclasses import dataclass, field
from enum import Enum, IntFlag

from gorgonzola.code_writer import CodeWriter
from gorgonzola.utils import to_snake_case, to_pascal_case, to_wire_name

if T.TYPE_CHECKING:
    from gorgonzola.post_process import TypeLayout, PostProcessedApiData

LICENSE = """\
// Copyright {year} {holder}
// SPDX-License-Identifier: {spdx}
//
// Generated via:
//   https://gitlab.freedesktop.org/mesa/mesa/-/tree/main/src/magma/gorgonzola
//
// Submit patches, do not hand-edit.

"""

MAX_LINE_LENGTH = 100
COMMON_INDENT = 4
SINGLE_INDENT = COMMON_INDENT * ' '


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
    # Backward-compatibility aliases
    'ffi_header': ApiAspectMask.FFI,
    'rust': ApiAspectMask.RUST_API,
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


class GorgonzolaTypeInstance(ABC):
    name: str
    aspect_mask: ApiAspectMask = ApiAspectMask.NONE
    allowed_aspects: T.Optional[ApiAspectMask] = None

    @abstractmethod
    def get_dependencies(self) -> T.List[str]:
        pass


@dataclass
class BaseRustTypeInfo(GorgonzolaTypeInstance):
    name: str
    num_bytes: int
    c_ffi_type: str

    def get_dependencies(self) -> T.List[str]:
        return []


RUST_TYPE_INFO_MAP: T.Dict[str, BaseRustTypeInfo] = {
    'u8': BaseRustTypeInfo('u8', 1, 'uint8_t'),
    'i8': BaseRustTypeInfo('i8', 1, 'int8_t'),
    'u16': BaseRustTypeInfo('u16', 2, 'uint16_t'),
    'i16': BaseRustTypeInfo('i16', 2, 'int16_t'),
    'u32': BaseRustTypeInfo('u32', 4, 'uint32_t'),
    'i32': BaseRustTypeInfo('i32', 4, 'int32_t'),
    'u64': BaseRustTypeInfo('u64', 8, 'uint64_t'),
    'i64': BaseRustTypeInfo('i64', 8, 'int64_t'),
    'f64': BaseRustTypeInfo('f64', 8, 'double'),
    'usize': BaseRustTypeInfo('usize', 8, 'uintptr_t'),
    'c_void': BaseRustTypeInfo('c_void', 8, 'void*'),
    'uint8_t': BaseRustTypeInfo('u8', 1, 'uint8_t'),
    'int8_t': BaseRustTypeInfo('i8', 1, 'int8_t'),
    'uint16_t': BaseRustTypeInfo('u16', 2, 'uint16_t'),
    'int16_t': BaseRustTypeInfo('i16', 2, 'int16_t'),
    'uint32_t': BaseRustTypeInfo('u32', 4, 'uint32_t'),
    'int32_t': BaseRustTypeInfo('i32', 4, 'int32_t'),
    'uint64_t': BaseRustTypeInfo('u64', 8, 'uint64_t'),
    'int64_t': BaseRustTypeInfo('i64', 8, 'int64_t'),
    'double': BaseRustTypeInfo('f64', 8, 'double'),
    'uintptr_t': BaseRustTypeInfo('usize', 8, 'uintptr_t'),
    'void*': BaseRustTypeInfo('c_void', 8, 'void*'),
}


@dataclass
class GorgonzolaConstant(GorgonzolaTypeInstance):
    name: str
    type_name: str
    value: int

    def get_dependencies(self) -> T.List[str]:
        return [self.type_name]


@dataclass
class GorgonzolaEnumEntry:
    name: str
    value: int


@dataclass
class GorgonzolaBaseEnum(GorgonzolaTypeInstance):
    name: str
    type_info: BaseRustTypeInfo
    entries: T.List[GorgonzolaEnumEntry]

    def get_dependencies(self) -> T.List[str]:
        return []


class GorgonzolaEnum(GorgonzolaBaseEnum):
    pass


class GorgonzolaFlag(GorgonzolaBaseEnum):
    pass


@dataclass
class GorgonzolaApiObject(GorgonzolaTypeInstance):
    name: str

    def get_dependencies(self) -> T.List[str]:
        return []


@dataclass
class GorgonzolaConstantArray(GorgonzolaTypeInstance):
    name: str
    element_type: GorgonzolaTypeInstance
    array_size_name: str

    def get_dependencies(self) -> T.List[str]:
        deps: T.List[str] = []
        # array: [u32; SIZE] or array: [PROP; size]
        if not isinstance(self.element_type, BaseRustTypeInfo):
            deps.append(self.element_type.name)
        # array: [u32; SIZE] or array: [u32; 16]
        if not self.array_size_name.isdigit():
            deps.append(self.array_size_name)
        return deps


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
    initial_offset: int = 0
    wire_struct: T.Optional[GorgonzolaStruct] = None

    def get_dependencies(self) -> T.List[str]:
        deps: T.List[str] = []
        for m in self.members:
            if not isinstance(m.type, BaseRustTypeInfo):
                deps.append(m.type.name)
            deps.extend(m.type.get_dependencies())
        return deps


@dataclass
class GorgonzolaExtensibleStruct(GorgonzolaStruct):
    stype: str = ''
    stype_value: int = 0
    extends: T.Optional[str] = None
    rust_member_name: T.Optional[str] = None
    initial_offset: int = 16


class CommandKind(Enum):
    DESTRUCTOR = 'destructor'
    STATIC_FUNCTION = 'static_function'
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
        if not self.inputs_const or not isinstance(self.inputs_const[0].type, GorgonzolaApiObject):
            return CommandKind.STATIC_FUNCTION
        if len(self.inputs_mut) == 1 and isinstance(self.inputs_mut[0].type, GorgonzolaApiObject):
            return CommandKind.OBJECT_CREATION
        return CommandKind.METHOD_CALL

    @property
    def inputs_const(self) -> T.List[GorgonzolaStructMember]:
        return [p for p in self.inputs if p.direction == Direction.IN]

    @property
    def inputs_mut(self) -> T.List[GorgonzolaStructMember]:
        return [p for p in self.inputs if p.direction == Direction.INOUT]


class HandleConversionConfig(T.TypedDict, total=False):
    rust_type: str
    expr: str
    default: str


class FfiConfig(T.TypedDict, total=False):
    crate_name: str
    custom_imports: T.List[str]
    exclude_imports: T.List[str]
    status_types: T.List[str]
    structure_type_header: str
    structure_type_enum: str
    handle_conversions: T.Dict[str, T.Union[str, HandleConversionConfig]]


class EncoderConfig(T.TypedDict, total=False):
    protocol_import: str
    custom_imports: T.List[str]


class DecoderConfig(T.TypedDict, total=False):
    protocol_import: str
    custom_imports: T.List[str]


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
    commands: T.Dict[str, GorgonzolaCommand] = field(default_factory=dict)
    ffi_config: FfiConfig = field(default_factory=FfiConfig)
    encoder_config: EncoderConfig = field(default_factory=EncoderConfig)
    decoder_config: DecoderConfig = field(default_factory=DecoderConfig)

    def add_type(self, obj: GorgonzolaTypeInstance) -> None:
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

    def find_by_name(self, name: str) -> T.Optional[GorgonzolaTypeInstance]:
        if name == 'void':
            return BaseRustTypeInfo('void', 0, 'void')
        return (
            RUST_TYPE_INFO_MAP.get(name)
            or self.structs.get(name)
            or self.extensible_structs.get(name)
            or self.api_objects.get(name)
            or self.enums.get(name)
            or self.flags.get(name)
            or self.arrays.get(name)
            or self.constants.get(name)
        )

    def resolve_type_name(self, name: str) -> GorgonzolaTypeInstance:
        obj = self.find_by_name(name)
        if obj is not None:
            return obj
        raise ValueError(f'Unable to resolve type name: {name}')

    def resolve_constant_to_int(self, const_name: str) -> T.Optional[int]:
        if const_name.isdigit():
            return int(const_name)
        const = self.constants.get(const_name)
        return const.value if const else None



class ApiName:
    def __init__(self, api_name: str) -> None:
        self.snake: str = to_snake_case(api_name)
        self.pascal: str = to_pascal_case(api_name)


class CommonGenerator(ABC):
    ASPECT_MASK: ApiAspectMask = ApiAspectMask.NONE

    def __init__(
        self,
        full_path: Path,
        copyright_info: CopyrightInfo,
        api_name: str,
        api_data: PostProcessedApiData,
    ) -> None:
        self.full_path = full_path
        self.copyright_info = copyright_info
        self.api_name = ApiName(api_name)
        self.api_data = api_data
        self.code_writer = CodeWriter(full_path)

    def emit(self) -> None:
        self._emit_internal()
        self.code_writer.write_file()

    def dispatch_type(self, obj: GorgonzolaTypeInstance) -> None:
        match obj:
            case GorgonzolaConstant():
                self.emit_constant(obj)
            case GorgonzolaApiObject():
                self.emit_api_object(obj)
            case GorgonzolaEnum():
                self.emit_enum(obj)
            case GorgonzolaFlag():
                self.emit_flag(obj)
            case GorgonzolaExtensibleStruct():
                self.emit_extensible_struct(obj)
            case GorgonzolaStruct():
                self.emit_struct(obj)
            case _:
                pass

    def dispatch_command(self, obj: GorgonzolaCommand) -> None:
        self.emit_command(obj)

    def emit_constant(self, obj: GorgonzolaConstant) -> None:
        pass

    def emit_api_object(self, obj: GorgonzolaApiObject) -> None:
        pass

    def emit_enum(self, obj: GorgonzolaEnum) -> None:
        pass

    def emit_flag(self, obj: GorgonzolaFlag) -> None:
        self.emit_enum(obj)

    def emit_struct(self, obj: GorgonzolaStruct) -> None:
        pass

    def emit_extensible_struct(self, obj: GorgonzolaExtensibleStruct) -> None:
        self.emit_struct(obj)

    def emit_command(self, obj: GorgonzolaCommand) -> None:
        pass

    @abstractmethod
    def _emit_internal(self) -> None:
        pass

    def _license(self) -> str:
        c = self.copyright_info
        return LICENSE.format(year=c.year, holder=c.holder, spdx=c.spdx)


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
        case GorgonzolaBaseEnum(name=name) | GorgonzolaStruct(name=name):
            return name
        case _:
            return type_inst.name
