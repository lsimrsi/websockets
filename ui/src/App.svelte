<script lang="ts">
  import { socket } from "./stores";
  import { onMount } from "svelte";

  let name = "";
  let message = "";

  onMount(async () => {
    if ($socket != null) return;

    let newSocket = new WebSocket("ws://127.0.0.1:8080/ws");

    newSocket.onopen = function (this: WebSocket, ev: Event) {
      console.log("Connected");
      this.send("hello from client");
    };

    newSocket.onclose = function () {
      console.log("Disconnected");
      $socket = null;
    };

    newSocket.onmessage = function (res: any) {
      console.log("data", res.data);
    };

    $socket = newSocket;
  });

  function onSubmit(e) {
    e.preventDefault();
    if ($socket == null) {
      console.log("Socket is null");
      return;
    }
    $socket.send(JSON.stringify({ name, message }));
  }
</script>

<main>
  <div>
    <p />
  </div>

  <form class="flex flex-col bg-yellow-400" on:submit={onSubmit}>
    <label for="name">Enter name:</label>
    <input id="name" bind:value={name} />

    <label for="chat-message">Enter message:</label>
    <textarea id="chat-message" bind:value={message} />
    <input type="submit" value="Send" />
  </form>
</main>

<style>
</style>
