<script>
  import { appWindow } from "@tauri-apps/plugin-window";
  import { invoke } from "@tauri-apps/api";

  export let outlet_socket_addr;

  let email;
  let error = null;

  async function submit() {
    await invoke("plugin:invitations|create_service_invitation", {
      outletSocketAddr: outlet_socket_addr,
      recipientEmail: email,
    })
      .then(() => appWindow.close())
      .catch((err) => {
        console.error(err);
        error = err;
      });
  }
  async function cancel() {
    await appWindow.close();
  }
</script>

<div class="mb-4 border-b pb-2 text-xl font-bold">
  Service details
  <h1 class="text-sm font-light">Sharing {outlet_socket_addr}</h1>
</div>
<div class="grid gap-4">
  <div class="flex items-start">
    <div class="mx-auto max-w-md flex-1">
      <div class="font-bold">Email Address</div>
      <p class="text-sm text-gray-500">
        Recipient will need to install the Ockam app, and enroll using this address.
      </p>
    </div>
    <div class="flex-1">
      <input
        type="email"
        class="w-full border-none bg-transparent px-4 text-right text-base focus:outline-none"
        placeholder="user@example.com"
        bind:value={email}
      />
    </div>
  </div>
</div>
<hr class="my-4" />
{#if error}
  <p class="mb-4 font-bold text-red-700">
    {error}
  </p>
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
