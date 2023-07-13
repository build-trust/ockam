<script>
    import { invoke } from '@tauri-apps/api/tauri'
    import { listen, emit } from '@tauri-apps/api/event';

    let tcp_outlets = invoke('tcp_outlet_list');

    async function create() {
        await invoke('tcp_outlet_create');
    }

    (async () => {
        await listen('app/tcp_outlets/on_update', (_event) => {
            tcp_outlets = invoke('tcp_outlet_list');
        });
    })();
</script>

<section>
    <h2>TCP Outlets</h2>
    <div>
        <button on:click="{create}">Create...</button>
    </div>
    <div>
        <ul>
            {#await tcp_outlets}
                <p>Fetching TCP outlets...</p>
            {:then items}
                {#each items as i}
                    <li>
                        <p>{i.worker_address} to {i.tcp_addr}</p>
                    </li>
                {/each}
            {:catch error}
                <p>{error.message}</p>
            {/await}
        </ul>
    </div>
</section>
