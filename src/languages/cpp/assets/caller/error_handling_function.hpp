if (lines.size() >= 2 && lines[0] == "E") {
    throw std::runtime_error("FFI call failed");
}
throw std::runtime_error("Unexpected bridge response");
