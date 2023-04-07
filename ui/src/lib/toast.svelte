<script lang="ts">
  import { ToastType, type ToastItem } from "../interfaces";

  import { quintOut } from "svelte/easing";
  import { crossfade } from "svelte/transition";
  import { flip } from "svelte/animate";

  const [send, receive] = crossfade({
    fallback(node, params) {
      const style = getComputedStyle(node);
      const transform = style.transform === "none" ? "" : style.transform;

      return {
        duration: 600,
        easing: quintOut,
        css: (t) => `
					transform: ${transform} scale(${t});
					opacity: ${t}
				`,
      };
    },
  });

  export let toast: ToastItem;

  // let anim = "300ms ease-out 0s 1 normal none running ";
  // const animation = toast.show ? anim + "fadeIn" : anim + "fadeOut";
  // const top = props.index * 100;

  let bg = "";
  switch (toast.type) {
    case ToastType.Network:
      bg = "bg-yellow-400 border-yellow-600/50";
      break;
    case ToastType.Info:
      bg = "bg-blue-400 border-blue-600/50";
      break;
    case ToastType.Success:
      bg = "bg-green-400 border-green-600/50";
      break;
    case ToastType.Redirect:
      bg = "bg-orange-400 border-orange-600/50";
      break;
    default:
      bg = "bg-red-400 border-red-600/50";
  }
</script>

<div
  in:receive={{ key: toast.uuid }}
  out:send={{ key: toast.uuid }}
  class={`flex right-0 w-96 transition-[top] rounded-bl-lg rounded-tr-lg mb-2 border shadow bg-white`}
>
  <div class={`flex rounded-bl-lg ${bg}`}>
    <span
      class={`m-auto text-center text-[1.5rem] text-stone-800 font-bold p-2`}
    >
      {toast.type}
    </span>
  </div>
  <div class="flex text-[1.2rem] text-stone-800 p-2">
    <span class="m-auto">{toast.message}</span>
  </div>
</div>
