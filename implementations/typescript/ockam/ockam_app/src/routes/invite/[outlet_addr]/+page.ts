/** @type {import('./$types').EntryGenerator}
 * Since this page has a dynamic address we need to specify the entries function
 * so that it can be pre-rendered.
 * See: https://kit.svelte.dev/docs/page-options#prerender-troubleshooting
 */
export function entries() {
  return [
    { outlet_addr: '12345' },
  ];
}

export const load = ({ params }) => {
  return {
    outlet_addr: params.outlet_addr,
  };
};
