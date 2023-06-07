<script>
  import Web from "svelte-material-icons/Web.svelte"
  import GitHub from "svelte-material-icons/Github.svelte"
  import Database from "svelte-material-icons/DatabaseOutline.svelte"
  import App from "svelte-material-icons/ApplicationBracesOutline.svelte"
  import Connect from "svelte-material-icons/LanConnect.svelte"
  import Disconnect from "svelte-material-icons/LanDisconnect.svelte"
  import { ProgressRadial } from '@skeletonlabs/skeleton';

  let build_synonyms = [
    'assemble',
    'construct',
    'make',
    'raise',
    'establish',
    'start',
    'begin',
    'secure',
    'collect',
    'gather',
    'rally'
  ];
  let trust_synonyms = [
    'confidence',
    'certainty',
    'assurance',
    'security',
    'care',
    'conviction',
    'sureness'
  ]
  let types = [
    'database',
    'application',
    'github',
    'web',
  ]
  let states = [
    'in',
    'out',
    'off'
  ]
  let Icons = {
    'database': Database,
    'application': App,
    'github': GitHub,
    'web': Web
  }
  const CONNECTED_STATES = ['in', 'out']


  let name = `${build_synonyms[Math.floor(build_synonyms.length * Math.random())]}-${trust_synonyms[Math.floor(trust_synonyms.length * Math.random())]}-${Math.floor(Math.random()*100)}`
  let portal_type = types[Math.floor(Math.random() * types.length)]
  let state = states[Math.floor(Math.random() * states.length)]
  let icon = Icons[portal_type]
  let loading = true
  let display = false
  let connecting = false

  async function show() {
    await new Promise(r => setTimeout(r, 2000 * Math.random()))
    loading = false
    await new Promise(r => setTimeout(r, 600 * Math.random()))
    display = true
  }

  function connect() {
    state = CONNECTED_STATES[Math.floor(Math.random() * CONNECTED_STATES.length)] 
    connecting = false
  }

  function disconnect() {
    state = "off"
    connecting = false
  }

  function toggle() {
    if (connecting) return
    connecting = true
    setTimeout(() => {
      if (CONNECTED_STATES.includes(state)) {
        disconnect()
      } else {
        connect()
      }
    }, 3000)
  }

  function keyToggle() {
    toggle()
  }

  $: show()
</script>


<div class="item">
    {#if loading}
    <div>
      <div class="placeholder-circle w-8 animate-pulse" />
      <span class="flex-auto">
        <dt class="placeholder animate-pulse w-64 my-1"></dt>
        <dd class="placeholder animate-pulse w-32"></dd>
      </span>
    </div>
    {:else}
    <div class="container {display ? "done" : "loading" }">
      <svg>
        <defs>
          <linearGradient id="in" gradientTransform="rotate(90)">
            <stop offset="20%" stop-color="#4FDAB8" />
            <stop offset="90%" stop-color="#36A7C9" />
          </linearGradient>
          <linearGradient id="out" gradientTransform="rotate(90)">
            <stop offset="20%" stop-color="#dabb4f" />
            <stop offset="90%" stop-color="#c95d35" />
          </linearGradient>
          <linearGradient id="off" gradientTransform="rotate(90)">
            <stop offset="20%" stop-color="rgba(255, 255, 255, 0.2)" />
            <stop offset="90%" stop-color="rgba(255, 255, 255, 0.1)" />
          </linearGradient>
        </defs>
        {#if portal_type === 'database'}
        <Database color="url(#{state})" size="25px" />
        {:else if portal_type === 'application'}
          <App color="url(#{state})" size="25px" />
        {:else if portal_type === 'github'}
        <GitHub color="url(#{state})" size="25px" />
        {:else if portal_type === 'web'}
        <Web color="url(#{state})" size="25px" />
        {/if}
        
      </svg>
      <span class="flex-auto">
          <dt>{name}</dt>
          <dd>{portal_type}</dd>
      </span>
      <div class="connection-state {(state === "in" || state === "out") ? "connected" : "disconnected" }" on:click={toggle} on:keypress={keyToggle}>
        {#if connecting}
         <ProgressRadial width="w-6" />
        {:else}
          {#if state === "in" || state === "out"}
            <Disconnect class="disconnect"/>
          {:else}
            <Connect class="connect"/>
          {/if}
        {/if}
      </div>
    </div>
    {/if}
</div>

<style>
  .connection-state {
    transition: opacity 0.6s ease-in-out;
    opacity: 0.2;
  }
  .connection-state:hover {
    opacity: 1;
  }
  .connected:hover {
    color: #e6767e;
  }
  .disconnected:hover {
    color: #97e676;
  }
  .item {
    margin: 0;
    border-radius: 4px;
  }
  .item:hover {
    background-color: rgba(255, 255, 255, 0.05);
  }

  dt {
    color: rgba(255, 255, 255, 0.7);
  }
  
  dd {
    color: rgba(255, 255, 255, 0.4);
  }

  svg {
    width: 25px;
    height: 25px;
    margin: 0;
    padding: 0;
  }

  .container {
    transition: opacity 1s ease-in-out;

    opacity: 0;
    cursor: pointer;
  }
  .container.loading {
    opacity: 0;
  }
  .container.done {
    opacity: 1;
  }

</style>
