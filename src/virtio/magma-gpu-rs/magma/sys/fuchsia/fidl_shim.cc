#include <fidl/fuchsia.gpu.magma/cpp/wire.h>
#include <fidl/fuchsia.io/cpp/wire.h>
#include <lib/zx/channel.h>
#include <zircon/syscalls.h>
#include <zircon/syscalls/object.h>
#include <zircon/threads.h>
#include <zircon/types.h>

#include <cstddef>
#include <cstdint>
#include <cstring>
#include <string>

namespace {

struct __attribute__((packed)) fuchsia_io_dirent {
  uint64_t ino;
  uint8_t size;
  uint8_t type;
  char name[0];
};

constexpr size_t kSizeofDirentBeforeName = offsetof(fuchsia_io_dirent, name);

class PrimaryEventHandler : public fidl::WireSyncEventHandler<fuchsia_gpu_magma::Primary> {
 public:
  PrimaryEventHandler(uint64_t* messages_consumed, uint64_t* memory_imported)
      : messages_consumed_(messages_consumed), memory_imported_(memory_imported) {}

  void OnNotifyMessagesConsumed(
      fidl::WireEvent<fuchsia_gpu_magma::Primary::OnNotifyMessagesConsumed>* event) override {
    if (messages_consumed_) {
      *messages_consumed_ += event->count;
    }
  }

  void OnNotifyMemoryImported(
      fidl::WireEvent<fuchsia_gpu_magma::Primary::OnNotifyMemoryImported>* event) override {
    if (memory_imported_) {
      *memory_imported_ += event->bytes;
    }
  }

 private:
  uint64_t* messages_consumed_;
  uint64_t* memory_imported_;
};

}

extern "C" {

zx_status_t magma_fidl_enumerate_devices(const char* device_namespace,
                                         uint32_t dir_channel,
                                         uint32_t max_devices,
                                         uint32_t path_size,
                                         char* paths_out,
                                         uint32_t* count_out) {
  if (!device_namespace || !paths_out || !count_out || path_size == 0) {
    if (dir_channel != ZX_HANDLE_INVALID) {
      zx_handle_close(dir_channel);
    }
    return ZX_ERR_INVALID_ARGS;
  }

  fidl::ClientEnd<fuchsia_io::Directory> client_end{zx::channel{dir_channel}};
  fidl::WireSyncClient<fuchsia_io::Directory> client{std::move(client_end)};

  uint32_t device_count = 0;
  char* path_ptr = paths_out;

  std::string ns(device_namespace);
  if (ns.empty() || ns.back() != '/') {
    ns += "/";
  }

  while (true) {
    auto result = client->ReadDirents(fuchsia_io::wire::kMaxBuf);
    if (!result.ok()) {
      return result.status();
    }

    auto& response = result.value();
    if (response.s != ZX_OK) {
      return response.s;
    }

    fidl::VectorView<uint8_t> dirents_buffer = response.dirents;
    if (dirents_buffer.size() == 0) {
      *count_out = device_count;
      return ZX_OK;
    }

    for (size_t offset = 0;;) {
      if (offset + kSizeofDirentBeforeName > dirents_buffer.size()) {
        break;
      }

      auto entry = reinterpret_cast<const fuchsia_io_dirent*>(&dirents_buffer.data()[offset]);
      size_t entry_size = kSizeofDirentBeforeName + entry->size;
      if (offset + entry_size > dirents_buffer.size()) {
        break;
      }
      offset += entry_size;

      std::string entry_name(entry->name, entry->size);
      if (entry_name == "." || entry_name == "..") {
        continue;
      }

      if (device_count >= max_devices) {
        return ZX_ERR_NO_MEMORY;
      }

      std::string path = ns + entry_name;
      if (entry->type == static_cast<uint8_t>(fuchsia_io::wire::DirentType::kDirectory)) {
        path += "/device";
      }

      if (path.size() >= path_size) {
        return ZX_ERR_INVALID_ARGS;
      }

      std::memcpy(path_ptr, path.c_str(), path.size() + 1);
      path_ptr += path_size;
      device_count += 1;
    }
  }
}

uint64_t magma_fidl_get_current_thread_koid() {
  zx_handle_t thrd_handle = thrd_get_zx_handle(thrd_current());
  zx_info_handle_basic_t info{};
  zx_status_t status =
      zx_object_get_info(thrd_handle, ZX_INFO_HANDLE_BASIC, &info, sizeof(info), nullptr, nullptr);
  return (status == ZX_OK) ? info.koid : 0;
}

zx_status_t magma_fidl_device_query(uint32_t device_channel,
                                    uint64_t query_id,
                                    uint32_t* result_buffer_out,
                                    uint64_t* result_out) {
  fidl::UnownedClientEnd<fuchsia_gpu_magma::Device> client_end(device_channel);
  auto result = fidl::WireCall(client_end)->Query(fuchsia_gpu_magma::wire::QueryId(query_id));
  if (!result.ok()) {
    return result.status();
  }
  if (result->is_error()) {
    return result->error_value();
  }

  if (result->value()->is_buffer_result()) {
    if (!result_buffer_out) {
      return ZX_ERR_INVALID_ARGS;
    }
    *result_buffer_out = result->value()->buffer_result().release();
    if (result_out) {
      *result_out = 0;
    }
    return ZX_OK;
  }

  if (result->value()->is_simple_result()) {
    if (!result_out) {
      return ZX_ERR_INVALID_ARGS;
    }
    *result_out = result->value()->simple_result();
    if (result_buffer_out) {
      *result_buffer_out = ZX_HANDLE_INVALID;
    }
    return ZX_OK;
  }

  return ZX_ERR_INTERNAL;
}

zx_status_t magma_fidl_device_connect2(uint32_t device_channel,
                                       uint64_t client_id,
                                       uint32_t* primary_channel_out,
                                       uint32_t* notification_channel_out) {
  if (!primary_channel_out || !notification_channel_out) {
    return ZX_ERR_INVALID_ARGS;
  }

  auto primary_endpoints = fidl::CreateEndpoints<fuchsia_gpu_magma::Primary>();
  if (!primary_endpoints.is_ok()) {
    return primary_endpoints.status_value();
  }

  auto notification_endpoints = fidl::CreateEndpoints<fuchsia_gpu_magma::Notification>();
  if (!notification_endpoints.is_ok()) {
    return notification_endpoints.status_value();
  }

  fidl::UnownedClientEnd<fuchsia_gpu_magma::Device> client_end(device_channel);
  auto result = fidl::WireCall(client_end)
                    ->Connect2(client_id,
                               std::move(primary_endpoints->server),
                               std::move(notification_endpoints->server));
  if (!result.ok()) {
    return result.status();
  }

  *primary_channel_out = primary_endpoints->client.TakeChannel().release();
  *notification_channel_out = notification_endpoints->client.TakeChannel().release();
  return ZX_OK;
}

zx_status_t magma_fidl_primary_enable_flow_control(uint32_t primary_channel) {
  fidl::UnownedClientEnd<fuchsia_gpu_magma::Primary> client_end(primary_channel);
  return fidl::WireCall(client_end)->EnableFlowControl().status();
}

zx_status_t magma_fidl_primary_flush(uint32_t primary_channel) {
  fidl::UnownedClientEnd<fuchsia_gpu_magma::Primary> client_end(primary_channel);
  return fidl::WireCall(client_end)->Flush().status();
}

zx_status_t magma_fidl_primary_handle_one_event(uint32_t primary_channel,
                                                uint64_t* messages_consumed_out,
                                                uint64_t* memory_imported_out) {
  if (messages_consumed_out) {
    *messages_consumed_out = 0;
  }
  if (memory_imported_out) {
    *memory_imported_out = 0;
  }
  PrimaryEventHandler handler(messages_consumed_out, memory_imported_out);
  fidl::UnownedClientEnd<fuchsia_gpu_magma::Primary> client_end(primary_channel);
  fidl::Status status = handler.HandleOneEvent(client_end);
  if (!status.ok() && status.reason() == fidl::Reason::kUnexpectedMessage) {
    return ZX_ERR_INTERNAL;
  }
  return status.status();
}

}
