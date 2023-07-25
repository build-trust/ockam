<script>
  import { invoke } from '@tauri-apps/api/tauri';

  let service = "/service/outlet";
  let port = "10000";

  async function submit() {
    await invoke('tcp_outlet_create', {service: service, port: port});
  }
</script>

<div>
  <h1>Service details</h1>
  <div>
    <label for="name">Service that you want to share:</label>
    <div>
      <input id="name" placeholder="{service}" bind:value="{service}" />
    </div>
  </div>
  <div>
    <label for="name">Service port:</label>
    <div>
      <input id="port" placeholder="{port}" bind:value="{port}" />
    </div>
  </div>
  <button on:click="{submit}">Create</button>
</div>

<style>
</style>
