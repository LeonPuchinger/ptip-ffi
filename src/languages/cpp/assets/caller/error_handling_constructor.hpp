throw std::runtime_error("Unexpected bridge response");
}
const auto value = ptip_ffi::decode_parameter_line(lines[2]);
if (value.kind != ptip_ffi::ParameterKind::Reference) {
    throw std::runtime_error("Constructor did not return a reference");
}
this->uuid = value.value;
