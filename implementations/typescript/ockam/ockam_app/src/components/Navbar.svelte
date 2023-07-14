<script>

import logo from "../assets/sygnet_white.svg"

let links = [
  {name: "Enroll", path: "/"},
  {name: "Outlets", path: "/"},
  {name: "Share", path: "/"},
  {name: "Reset", path: "/"},
]

</script>

<h1 class="align-middle max-w-full text-4xl text-center text-primary-ockam">
  <img alt="Ockam Sygnet" class="inline max-h-12 max-w-12" src={logo} />
  Ockam
</h1>
<h2 class="divide-natural-light divide-opacity-50 divide-x max-w-full text-center">
  {#each links as link}
  <a class="hover:underline px-0.5 text-secondary-avocado" href={link.path}>{link.name}</a>
  {/each}
</h2>
