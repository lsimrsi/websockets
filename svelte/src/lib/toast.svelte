<script lang="ts">
  import { ToastType, type ToastItem } from "../interfaces";

  export let toast: ToastItem;

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

<div class={`flex rounded-bl-lg ${bg}`}>
  <span class={`m-auto text-center text-[1.5rem] text-stone-800 font-bold p-2`}>
    {toast.type}
  </span>
</div>
<div class="flex text-[1.2rem] text-stone-800 p-2">
  <span class="m-auto">{toast.message}</span>
</div>
