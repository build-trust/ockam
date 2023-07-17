<script>
    import {invoke} from '@tauri-apps/api/tauri'

    let done = false;

    async function reset() {
        await invoke('reset')
    }
</script>

<div>
    <button on:click="{reset}">Reset...</button>
</div>
