<script>
  import { getCurrent } from "@tauri-apps/plugin-window";
  import { invoke } from "@tauri-apps/api/tauri";

  let service = "/service/outlet";
  let port = "10000";
  let error_message = "";

  async function submit() {
    error_message = "";
    await invoke("tcp_outlet_create", { service: service, port: port })
      .then(close)
      .catch((error) => {
        error_message = error;
      });
  }

  function cancel() {
    close();
  }

  function close() {
    getCurrent().close();
  }
</script>

<div class="border-b mb-4 pb-2 font-bold text-xl">Service details</div>
<div class="grid gap-4">
  <div class="flex items-start">
    <div class="flex-1">
      <div class="font-bold">Name</div>
      <p class="text-sm text-gray-500">Name of the service you want to share</p>
    </div>
    <div class="flex-1">
      <input
        type="text"
        class="w-full px-4 bg-transparent border-none focus:outline-none text-right"
        placeholder={service}
        bind:value={service}
      />
    </div>
  </div>
  <div class="flex items-start">
    <div class="flex-1">
      <div class="font-bold">Port</div>
      <p class="text-sm text-gray-500">Choose a port for the service</p>
    </div>
    <div class="flex-1">
      <input
        type="text"
        class="w-full px-4 bg-transparent border-none focus:outline-none text-right"
        placeholder={port}
        bind:value={port}
      />
    </div>
  </div>
</div>
<hr class="my-4" />
{#if error_message}
  <div class="mb-2 text-red-500 text-sm">{error_message}</div>
{/if}
<div class="flex justify-end">
  <button
    class="px-2 py-1 mr-2 bg-gray-300 text-gray-700 rounded hover:bg-gray-400"
    on:click={cancel}>Cancel</button
  >
  <button
    class="px-2 py-1 bg-blue-500 text-white rounded hover:bg-blue-600"
    on:click={submit}>Create</button
  >
</div>
