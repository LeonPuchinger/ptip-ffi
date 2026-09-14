#include "hashmap.grpc.pb.h"
#include "input/cpp/index.hpp"
#include <grpcpp/grpcpp.h>
#include <memory>
#include <iostream>
#include <cstdlib>
#include <string>
#include <unordered_map>
#include <random>

class Service final : public hashmap::HashMapService::Service {
    std::unordered_map<std::string, HashMap<std::string, int>> maps;
public:
    grpc::Status Create(grpc::ServerContext*, const hashmap::Empty*, hashmap::Handle* response) override {
        const auto id = std::to_string(maps.size()) + "-" + std::to_string(std::random_device{}());
        maps.emplace(id, HashMap<std::string, int>{});
        response->set_id(id);
        return grpc::Status::OK;
    }
    grpc::Status Set(grpc::ServerContext*, const hashmap::Entry* request, hashmap::Empty*) override {
        maps.at(request->handle()).set(request->key(), static_cast<int>(request->value()));
        return grpc::Status::OK;
    }
    grpc::Status Get(grpc::ServerContext*, const hashmap::Key* request, hashmap::Value* response) override {
        response->set_value(maps.at(request->handle()).get(request->key()));
        return grpc::Status::OK;
    }
    grpc::Status Has(grpc::ServerContext*, const hashmap::Key* request, hashmap::BoolValue* response) override {
        response->set_value(maps.at(request->handle()).has(request->key()));
        return grpc::Status::OK;
    }
};

int main() {
    const std::string address = "127.0.0.1:" + std::string(getenv("FFI_GRPC_PORT") ? getenv("FFI_GRPC_PORT") : "50051");
    Service service;
    grpc::ServerBuilder builder;
    builder.AddListeningPort(address, grpc::InsecureServerCredentials());
    builder.RegisterService(&service);
    auto server = builder.BuildAndStart();
    std::cout << "ready" << std::endl;
    server->Wait();
}
