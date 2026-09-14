#pragma once

#include <cmath>
#include <map>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

template <typename K, typename V>
class HashMap {
public:
    void set(const K &key, V value) {
        data[key] = std::move(value);
    }

    V get(const K &key) const {
        auto it = data.find(key);
        if (it == data.end()) {
            throw std::out_of_range("key not found");
        }
        return it->second;
    }

    bool has(const K &key) const {
        return data.find(key) != data.end();
    }

    bool remove(const K &key) {
        return data.erase(key) > 0;
    }

private:
    std::map<K, V> data;
};
