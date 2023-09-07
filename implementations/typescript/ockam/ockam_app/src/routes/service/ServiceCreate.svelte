<script>
  import { getCurrent } from "@tauri-apps/plugin-window";
  import { invoke } from "@tauri-apps/api/tauri";

  let service = "";
  let address = "localhost:10000";
  let email = "";
  let error_message = "";

  async function submit() {
    error_message = "";
    await invoke("plugin:shared_service|tcp_outlet_create", {
      service: service,
      address: address,
      email: email,
    })
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

<div class="mb-4 border-b pb-2 text-xl font-bold">Service details</div>
<div class="grid gap-4">
  <div class="flex items-start">
    <div class="flex-1">
      <div class="font-bold">Name</div>
      <p class="text-sm text-gray-500">Name of the service you want to share</p>
    </div>
    <div class="flex-1">
      <input
        type="text"
        class="w-full border-none bg-transparent px-4 text-right text-base focus:outline-none"
        placeholder="service"
        bind:value={service}
      />
    </div>
  </div>
  <div class="flex items-start">
    <div class="flex-1">
      <div class="font-bold">Address</div>
      <p class="text-sm text-gray-500">Choose an address for the service</p>
    </div>
    <div class="flex-1">
      <input
        type="text"
        class="w-full border-none bg-transparent px-4 text-right text-base focus:outline-none"
        placeholder={address}
        bind:value={address}
      />
    </div>
  </div>
  <div class="flex items-start">
    <div class="flex-1">
      <div class="font-bold">Share</div>
      <p class="text-sm text-gray-500">
        Optionally, send an invitation to share this service
      </p>
    </div>
    <div class="min-w-[60%] flex-1">
      <input
        type="email"
        class="w-full border-none bg-transparent px-4 text-right text-base focus:outline-none"
        placeholder="recipient@mail.com"
        bind:value={email}
      />
    </div>
  </div>
</div>
<hr class="my-4" />
{#if error_message}
  <div class="mb-2 text-sm text-red-500">{error_message}</div>
{/if}
<div class="flex justify-end">
  <button
    class="mr-2 rounded bg-gray-300 px-2 py-1 text-base text-gray-700 hover:bg-gray-400"
    on:click={cancel}>Cancel</button
  >
  <button
    class="rounded bg-blue-500 px-2 py-1 text-base text-white hover:bg-blue-600"
    on:click={submit}>Create</button
  >
</div>
