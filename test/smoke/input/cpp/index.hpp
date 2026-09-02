#pragma once

#include <cmath>
#include <map>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

inline std::string trim_whitespace(const std::string &str) {
    const auto begin = str.find_first_not_of(" \t\r\n");
    if (begin == std::string::npos) {
        return "";
    }
    const auto end = str.find_last_not_of(" \t\r\n");
    return str.substr(begin, end - begin + 1);
}

struct Point {
    float x;
    float y;

    Point(float x_value, float y_value) {
        x = x_value;
        y = y_value;
    }

    double distance_to_origin() const {
        return std::sqrt(static_cast<double>(x * x + y * y));
    }
};

double takes_point(const Point &point) {
    return point.distance_to_origin();
}

template <typename T>
struct LinkedListNode {
    T value;
    LinkedListNode<T> *next;

    LinkedListNode(T value_value, LinkedListNode<T> *next_value = nullptr) {
        value = std::move(value_value);
        next = next_value;
    }
};

template <typename T>
class LinkedList {
public:
    LinkedList() {
        this->root = nullptr;
    }

    void append(T value) {
        auto *new_node = new LinkedListNode<T>(std::move(value));
        if (root == nullptr) {
            root = new_node;
            return;
        }

        auto *current = root;
        while (current->next != nullptr) {
            current = current->next;
        }
        current->next = new_node;
    }

    T get(std::size_t index) const {
        auto *current = root;
        std::size_t count = 0;
        while (current != nullptr) {
            if (count == index) {
                return current->value;
            }
            count++;
            current = current->next;
        }
        throw std::out_of_range("index out of bounds");
    }

private:
    LinkedListNode<T> *root;
};

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
