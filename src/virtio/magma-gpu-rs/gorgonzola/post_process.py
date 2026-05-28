# Copyright 2026 The Magma GPU Project
# SPDX-License-Identifier: MIT

from __future__ import annotations
from dataclasses import dataclass
import typing as T

from gorgonzola.common import (
    ApiAspectMask,
    ApiData,
    BaseRustTypeInfo,
    CodecConfig,
    Direction,
    FfiConfig,
    GorgonzolaApiObject,
    GorgonzolaBaseEnum,
    GorgonzolaCommand,
    GorgonzolaConstant,
    GorgonzolaConstantArray,
    GorgonzolaDynamicArray,
    GorgonzolaEnum,
    GorgonzolaEnumEntry,
    GorgonzolaExtensibleStruct,
    GorgonzolaStruct,
    GorgonzolaStructMember,
    GorgonzolaTypeInstance,
    LayoutEntry,
    RUST_TYPE_INFO_MAP,
    STRUCTURE_TYPE_HEADER_SIZE,
    TypeLayout,
    U32_BYTE_SIZE,
    to_rust_protocol_type,
)
from gorgonzola.utils import normalize_command_name, to_wire_name

COMMAND_HEADER_SIZE: int = 8
POINTER_SIZE: int = 8
POINTER_ALIGNMENT: int = 8
API_OBJECT_PROTOCOL_SIZE: int = 4
API_OBJECT_PROTOCOL_ALIGNMENT: int = 4
DEFAULT_FIELD_ALIGNMENT: int = 4
PROTOCOL_STRUCT_ALIGNMENT: int = 8
MAX_PROTOCOL_ALIGNMENT: int = 8


def calculate_padding(offset: int, align: int) -> int:
    """Calculates padding bytes needed to align an offset to the specified alignment."""
    remainder = offset % align
    return (align - remainder) if remainder != 0 else 0


def calculate_trailing_padding(
    offset: int, struct_alignment: int = PROTOCOL_STRUCT_ALIGNMENT
) -> int:
    """Calculates trailing padding bytes needed to round struct size to struct alignment."""
    return calculate_padding(offset, struct_alignment)


def make_padding_entry(name: str, offset: int, size: int) -> LayoutEntry:
    """Creates a padding layout entry with the appropriate Rust protocol type."""
    pad_type = 'u32' if size == U32_BYTE_SIZE else f'[u8; {size}]'
    return LayoutEntry(
        name=name,
        type_name=pad_type,
        offset=offset,
        size=size,
        is_padding=True,
    )


def _member_has_api_objects(type_inst: GorgonzolaTypeInstance) -> bool:
    match type_inst:
        case GorgonzolaApiObject():
            return True
        case GorgonzolaConstantArray(element_type=elem) | GorgonzolaDynamicArray(element_type=elem):
            return _member_has_api_objects(elem)
        case GorgonzolaStruct(has_api_object=has_api):
            return has_api
        case _:
            return False


def compute_has_api_object(sorted_types: T.List[GorgonzolaTypeInstance]) -> None:
    """Determines whether structs contain ApiObjects directly or transitively."""
    for obj in sorted_types:
        match obj:
            case GorgonzolaStruct(members=members):
                obj.has_api_object = any(_member_has_api_objects(m.type) for m in members)
            case _:
                pass


def _member_has_dynamic_arrays(type_inst: GorgonzolaTypeInstance) -> bool:
    match type_inst:
        case GorgonzolaDynamicArray():
            return True
        case GorgonzolaConstantArray(element_type=elem):
            return _member_has_dynamic_arrays(elem)
        case GorgonzolaStruct(has_dynamic_array=has_dyn):
            return has_dyn
        case _:
            return False


def compute_has_dynamic_array(sorted_types: T.List[GorgonzolaTypeInstance]) -> None:
    """Determines whether structs contain DynamicArrays directly or transitively."""
    for obj in sorted_types:
        match obj:
            case GorgonzolaStruct(members=members):
                obj.has_dynamic_array = any(_member_has_dynamic_arrays(m.type) for m in members)
            case _:
                pass


def struct_has_api_objects(type_inst: GorgonzolaTypeInstance) -> bool:
    match type_inst:
        case GorgonzolaStruct(has_api_object=has_api, has_dynamic_array=has_dyn):
            return has_api or has_dyn
        case _:
            return False


def get_type_size(
    type_inst: GorgonzolaTypeInstance,
    is_protocol: bool = True,
) -> T.Optional[int]:
    """Returns the size of a type in protocol or FFI layout."""
    match type_inst:
        case BaseRustTypeInfo(num_bytes=num_bytes):
            return num_bytes
        case GorgonzolaBaseEnum(type_info=type_info):
            return type_info.num_bytes
        case GorgonzolaApiObject():
            return API_OBJECT_PROTOCOL_SIZE if is_protocol else POINTER_SIZE
        case GorgonzolaConstant(type_info=type_info):
            return type_info.num_bytes
        case GorgonzolaConstantArray(element_type=elem, array_size=count):
            elem_size = get_type_size(elem, is_protocol=is_protocol)
            return elem_size * count if elem_size is not None else None
        case GorgonzolaDynamicArray():
            return None if is_protocol else POINTER_SIZE
        case GorgonzolaStruct():
            layout = type_inst.protocol_layout if is_protocol else type_inst.ffi_layout
            return layout.size if layout else None
        case _:
            return None


def get_type_align(
    type_inst: GorgonzolaTypeInstance,
    is_protocol: bool = True,
) -> int:
    """Returns the alignment of a type in protocol or FFI layout."""
    match type_inst:
        case BaseRustTypeInfo(num_bytes=num_bytes):
            return min(num_bytes, MAX_PROTOCOL_ALIGNMENT)
        case GorgonzolaBaseEnum(type_info=type_info):
            return min(type_info.num_bytes, MAX_PROTOCOL_ALIGNMENT)
        case GorgonzolaApiObject():
            return API_OBJECT_PROTOCOL_ALIGNMENT if is_protocol else POINTER_ALIGNMENT
        case GorgonzolaConstant(type_info=type_info):
            return min(type_info.num_bytes, MAX_PROTOCOL_ALIGNMENT)
        case GorgonzolaConstantArray(element_type=elem):
            return get_type_align(elem, is_protocol=is_protocol)
        case GorgonzolaDynamicArray():
            return DEFAULT_FIELD_ALIGNMENT if is_protocol else POINTER_ALIGNMENT
        case GorgonzolaStruct():
            layout = type_inst.protocol_layout if is_protocol else type_inst.ffi_layout
            return layout.alignment if layout else DEFAULT_FIELD_ALIGNMENT
        case _:
            return DEFAULT_FIELD_ALIGNMENT


def layout_fields(
    fields: T.List[GorgonzolaStructMember],
    initial_offset: int = 0,
    is_protocol: bool = True,
) -> TypeLayout:
    """Calculates field offsets, padding, and struct layout"""
    entries: T.List[LayoutEntry] = []
    curr_offset = initial_offset
    pad_idx = 0
    max_align = (
        MAX_PROTOCOL_ALIGNMENT
        if (initial_offset > 0 and is_protocol)
        else DEFAULT_FIELD_ALIGNMENT
    )

    for f in fields:
        if is_protocol and isinstance(f.type, GorgonzolaDynamicArray):
            continue
        align = (
            POINTER_ALIGNMENT
            if f.is_pointer
            else get_type_align(f.type, is_protocol=is_protocol)
        )
        max_align = max(max_align, align)
        size = (
            POINTER_SIZE
            if f.is_pointer
            else get_type_size(f.type, is_protocol=is_protocol)
        )
        if size is None:
            raise ValueError(
                f"Cannot determine size of field '{f.name}' of type '{f.type.name}'"
            )

        pad_size = calculate_padding(curr_offset, align)
        if pad_size > 0:
            entries.append(make_padding_entry(f'_pad{pad_idx}', curr_offset, pad_size))
            pad_idx += 1
            curr_offset += pad_size

        entries.append(
            LayoutEntry(
                name=f.name,
                type_name=(
                    f'*mut {to_rust_protocol_type(f.type)}'
                    if f.is_pointer
                    else to_rust_protocol_type(f.type)
                ),
                offset=curr_offset,
                size=size,
                is_padding=False,
            )
        )
        curr_offset += size

    if is_protocol:
        trail_pad = calculate_trailing_padding(curr_offset, PROTOCOL_STRUCT_ALIGNMENT)
        if trail_pad > 0:
            entries.append(make_padding_entry('_padding', curr_offset, trail_pad))
            curr_offset += trail_pad
        struct_align = min(max_align, MAX_PROTOCOL_ALIGNMENT)
    else:
        struct_align = max_align
        trail_pad = calculate_padding(curr_offset, struct_align)
        curr_offset += trail_pad

    return TypeLayout(
        size=curr_offset,
        alignment=struct_align,
        entries=entries,
        initial_offset=initial_offset,
    )


def to_wire_member(
    member: GorgonzolaStructMember,
    u32_type: BaseRustTypeInfo,
) -> GorgonzolaStructMember:
    match member.type:
        case GorgonzolaApiObject():
            return GorgonzolaStructMember(
                name=member.name,
                type=u32_type,
                direction=Direction.IN,
                is_pointer=False,
            )
        case GorgonzolaConstantArray(
            element_type=elem,
            array_size_name=size_name,
            array_size=array_size,
            size_constant=size_constant,
        ):
            if isinstance(elem, GorgonzolaApiObject):
                wire_elem: GorgonzolaTypeInstance = u32_type
            elif isinstance(elem, GorgonzolaStruct) and elem.wire_struct is not None:
                wire_elem = elem.wire_struct
            else:
                wire_elem = elem

            if wire_elem != elem:
                wire_type = GorgonzolaConstantArray(
                    f'[{to_rust_protocol_type(wire_elem)}; {size_name}]',
                    wire_elem,
                    size_name,
                    array_size,
                    size_constant,
                )
                return GorgonzolaStructMember(
                    name=member.name,
                    type=wire_type,
                    direction=(
                        Direction.IN
                        if isinstance(elem, GorgonzolaApiObject)
                        else member.direction
                    ),
                    is_pointer=(
                        False if isinstance(elem, GorgonzolaApiObject) else member.is_pointer
                    ),
                )
            return member
        case GorgonzolaDynamicArray(
            element_type=elem,
            size_member_name=size_name,
        ):
            if isinstance(elem, GorgonzolaApiObject):
                wire_dyn_elem: GorgonzolaTypeInstance = u32_type
            elif isinstance(elem, GorgonzolaStruct) and elem.wire_struct is not None:
                wire_dyn_elem = elem.wire_struct
            else:
                wire_dyn_elem = elem

            if wire_dyn_elem != elem:
                wire_dyn_type = GorgonzolaDynamicArray(
                    f'Vec<{to_rust_protocol_type(wire_dyn_elem)};{size_name}>',
                    wire_dyn_elem,
                    size_name,
                )
                return GorgonzolaStructMember(
                    name=member.name,
                    type=wire_dyn_type,
                    direction=(
                        Direction.IN
                        if isinstance(elem, GorgonzolaApiObject)
                        else member.direction
                    ),
                    is_pointer=(
                        False if isinstance(elem, GorgonzolaApiObject) else member.is_pointer
                    ),
                )
            return member
        case GorgonzolaStruct(wire_struct=wire_struct) if wire_struct is not None:
            return GorgonzolaStructMember(
                name=member.name,
                type=wire_struct,
                direction=member.direction,
                is_pointer=member.is_pointer,
            )
        case _:
            return member


@dataclass
class PostProcessedApiData:
    ffi_config: FfiConfig
    encoder_config: CodecConfig
    decoder_config: CodecConfig
    sorted_types: T.List[GorgonzolaTypeInstance]
    commands: T.List[GorgonzolaCommand]
    extensible_structs: T.Dict[str, GorgonzolaExtensibleStruct]
    api_name: str = ''
    structure_type_enum: T.Optional[GorgonzolaEnum] = None

    def __init__(self, api_data: ApiData) -> None:
        self.api_name = api_data.api_name
        self.ffi_config = api_data.ffi_config
        self.encoder_config = api_data.encoder_config
        self.decoder_config = api_data.decoder_config
        self.commands = list(api_data.commands.values())
        self.extensible_structs = dict(api_data.extensible_structs)
        self.structure_type_enum = None

        self._topological_sort_types(api_data)
        self._propagate_aspects()
        compute_has_api_object(self.sorted_types)
        compute_has_dynamic_array(self.sorted_types)
        self._synthesize_wire_structs()
        self._compute_struct_layouts()
        self._compute_structure_type_enum()

    def _topological_sort_types(self, api_data: ApiData) -> None:
        """Topologically sorts all registered types in dependency order."""
        visited: T.Set[str] = set()
        visiting: T.Set[str] = set()
        result: T.List[GorgonzolaTypeInstance] = []

        def visit(obj: GorgonzolaTypeInstance) -> None:
            if obj.name in visited:
                return
            if obj.name in visiting:
                raise ValueError(f"Circular dependency detected for type '{obj.name}'")
            visiting.add(obj.name)
            for dep in obj.get_dependencies():
                visit(dep)
            visiting.remove(obj.name)
            visited.add(obj.name)
            result.append(obj)

        for obj in api_data.all_types():
            visit(obj)
        self.sorted_types: T.List[GorgonzolaTypeInstance] = result

    def _synthesize_wire_structs(self) -> None:
        u32_type = RUST_TYPE_INFO_MAP['u32']

        new_sorted_types: T.List[GorgonzolaTypeInstance] = []
        for obj in self.sorted_types:
            new_sorted_types.append(obj)
            match obj:
                case GorgonzolaStruct(has_api_object=True):
                    wire_name = to_wire_name(obj.name, self.api_name)
                    wire_members = [to_wire_member(m, u32_type) for m in obj.members]

                    wire_struct: GorgonzolaStruct
                    match obj:
                        case GorgonzolaExtensibleStruct():
                            ext_wire = GorgonzolaExtensibleStruct(
                                name=wire_name,
                                members=wire_members,
                                stype=obj.stype,
                                stype_value=obj.stype_value,
                                extends=obj.extends,
                                rust_member_name=obj.rust_member_name,
                                initial_offset=STRUCTURE_TYPE_HEADER_SIZE,
                            )
                            self.extensible_structs[wire_name] = ext_wire
                            wire_struct = ext_wire
                        case _:
                            wire_struct = GorgonzolaStruct(
                                name=wire_name,
                                members=wire_members,
                                initial_offset=0,
                            )

                    wire_struct.aspect_mask = ApiAspectMask.WIRE | ApiAspectMask.RUST_API
                    wire_struct.has_api_object = False
                    wire_struct.has_dynamic_array = obj.has_dynamic_array
                    obj.wire_struct = wire_struct

                    obj.aspect_mask &= ~(ApiAspectMask.WIRE | ApiAspectMask.RUST_API)
                    obj.aspect_mask |= ApiAspectMask.FFI

                    new_sorted_types.append(wire_struct)
                case _:
                    pass

        self.sorted_types = new_sorted_types

        for ext in self.extensible_structs.values():
            if (
                not ext.has_api_object
                and ext.extends is not None
                and isinstance(ext.extends.wire_struct, GorgonzolaExtensibleStruct)
            ):
                ext.extends = ext.extends.wire_struct

        for cmd in self.commands_for_aspect(ApiAspectMask.WIRE):
            cmd_pascal = normalize_command_name(cmd.name, self.api_name)
            wire_inputs = [to_wire_member(inp, u32_type) for inp in cmd.inputs]

            req_members = [m for m in wire_inputs if m.direction == Direction.IN]
            cmd.req_struct = GorgonzolaStruct(
                name=cmd_pascal,
                members=req_members,
                has_dynamic_array=any(_member_has_dynamic_arrays(m.type) for m in req_members),
                initial_offset=COMMAND_HEADER_SIZE,
            )
            cmd.req_struct.aspect_mask = ApiAspectMask.WIRE

            if cmd.response:
                resp_members = [m for m in wire_inputs if m.direction == Direction.INOUT]
                cmd.resp_struct = GorgonzolaStruct(
                    name=f'{cmd_pascal}Resp',
                    members=resp_members,
                    has_dynamic_array=any(_member_has_dynamic_arrays(m.type) for m in resp_members),
                    initial_offset=COMMAND_HEADER_SIZE,
                )
                cmd.resp_struct.aspect_mask = ApiAspectMask.WIRE

    def _compute_struct_layouts(self) -> None:
        for obj in self.sorted_types:
            if not isinstance(obj, GorgonzolaStruct):
                continue

            obj.ffi_abi_matches_protocol = not (obj.has_api_object or obj.has_dynamic_array)

            if obj.aspect_mask & ApiAspectMask.WIRE:
                obj.protocol_layout = layout_fields(
                    obj.members,
                    initial_offset=obj.initial_offset,
                    is_protocol=True,
                )

            if (obj.aspect_mask & ApiAspectMask.FFI) and obj.ffi_abi_matches_protocol:
                if obj.protocol_layout:
                    obj.ffi_layout = obj.protocol_layout
                else:
                    obj.ffi_layout = layout_fields(
                        obj.members,
                        initial_offset=obj.initial_offset,
                        is_protocol=False,
                    )

        for cmd in self.commands_for_aspect(ApiAspectMask.WIRE):
            if cmd.req_struct:
                cmd.req_struct.protocol_layout = layout_fields(
                    cmd.req_struct.members,
                    initial_offset=cmd.req_struct.initial_offset,
                    is_protocol=True,
                )
            if cmd.resp_struct:
                cmd.resp_struct.protocol_layout = layout_fields(
                    cmd.resp_struct.members,
                    initial_offset=cmd.resp_struct.initial_offset,
                    is_protocol=True,
                )

    def _propagate_type_dependencies(self) -> None:
        for item in reversed(self.sorted_types):
            for dep in item.get_dependencies():
                dep.aspect_mask |= item.aspect_mask

    def _propagate_aspects(self) -> None:
        for cmd in self.commands:
            for inp in cmd.inputs:
                inp.type.aspect_mask |= cmd.aspect_mask
            cmd.ret.aspect_mask |= cmd.aspect_mask

        self._propagate_type_dependencies()

        for ext in self.extensible_structs.values():
            if ext.aspect_mask == ApiAspectMask.NONE:
                parent = ext.extends
                while parent is not None:
                    ext.aspect_mask |= parent.aspect_mask
                    parent = parent.extends

        self._propagate_type_dependencies()

    def _compute_structure_type_enum(self) -> None:
        ext_structs = [
            obj for obj in self.sorted_types if isinstance(obj, GorgonzolaExtensibleStruct)
        ]
        if not ext_structs:
            self.structure_type_enum = None
            return

        entries: T.List[GorgonzolaEnumEntry] = []
        combined_mask = ApiAspectMask.NONE
        for ext in ext_structs:
            entries.append(GorgonzolaEnumEntry(name=ext.stype, value=ext.stype_value))
            combined_mask |= ext.aspect_mask

        enum_name = self.ffi_config.structure_type_enum
        u32_type = RUST_TYPE_INFO_MAP['u32']
        self.structure_type_enum = GorgonzolaEnum(
            name=enum_name,
            type_info=u32_type,
            entries=entries,
        )
        self.structure_type_enum.aspect_mask = combined_mask

    def extensions_for(self, struct_name: str) -> T.List[GorgonzolaExtensibleStruct]:
        return [
            s
            for s in self.extensible_structs.values()
            if s.extends is not None and s.extends.name == struct_name
        ]

    def types(self, aspect_mask: ApiAspectMask) -> T.Iterator[GorgonzolaTypeInstance]:
        for t in self.sorted_types:
            if t.aspect_mask & aspect_mask:
                yield t

    def constants(self, aspect_mask: ApiAspectMask) -> T.Iterator[GorgonzolaConstant]:
        for t in self.sorted_types:
            if isinstance(t, GorgonzolaConstant) and (t.aspect_mask & aspect_mask):
                yield t

    def extensible_structs_for_aspect(
        self, aspect_mask: ApiAspectMask
    ) -> T.Iterator[GorgonzolaExtensibleStruct]:
        for t in self.sorted_types:
            if isinstance(t, GorgonzolaExtensibleStruct) and (t.aspect_mask & aspect_mask):
                yield t

    def commands_for_aspect(
        self, aspect_mask: ApiAspectMask
    ) -> T.Iterator[GorgonzolaCommand]:
        for cmd in self.commands:
            if cmd.aspect_mask & aspect_mask:
                yield cmd

    def structure_type_enum_for_aspect(
        self, aspect_mask: ApiAspectMask
    ) -> T.Optional[GorgonzolaEnum]:
        if self.structure_type_enum is None:
            return None
        ext_structs = list(self.extensible_structs_for_aspect(aspect_mask))
        if not ext_structs:
            return None
        entries = [
            GorgonzolaEnumEntry(name=ext.stype, value=ext.stype_value)
            for ext in ext_structs
        ]
        return GorgonzolaEnum(
            name=self.structure_type_enum.name,
            type_info=self.structure_type_enum.type_info,
            entries=entries,
        )
