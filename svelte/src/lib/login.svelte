<script lang="ts">
  import { name } from "../stores";
  import { sendMessage } from "../utility-functions";
  import Input from "./input.svelte";

  function onSubmit(e: Event) {
    e.preventDefault();
    console.log("$name", $name);
    sendMessage({ msg_type: "RegisterName", data: $name });
  }
</script>

<form class="m-auto" on:submit={onSubmit}>
  <Input label="Please enter your name:" id="name" bind:value={$name} />
</form>
