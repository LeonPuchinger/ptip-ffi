throw std::runtime_error("Unexpected bridge response");
}
const auto constructor_value = ptip_ffi::decode_parameter_line(lines[2]);
if (constructor_value.kind != ptip_ffi::ParameterKind::Reference) {
    throw std::runtime_error("Constructor did not return a reference");
}
this->uuid = constructor_value.value;
