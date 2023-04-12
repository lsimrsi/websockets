<script setup lang="ts">

defineEmits(['update:modelValue'])
defineProps<{
  label: string;
  id: string;
  modelValue: string;
  classes?: string;
}>()

</script>

<template>
  <div class="flex flex-col">
    <label class="text-slate-600" for="id">{{label}}</label>
    <input class="`border p-2 ${classes}`" :id="id" :value="modelValue" @input="$emit('update:modelValue', ($event.target as HTMLInputElement).value)" />
  </div>
</template>
