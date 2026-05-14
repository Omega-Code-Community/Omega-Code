<script setup lang="ts">
import type { HTMLAttributes } from "vue"
import { cn } from "@/lib/utils"

const props = defineProps<{
  class?: HTMLAttributes["class"]
}>()
</script>

<template>
  <main
    data-slot="sidebar-inset"
    :class="cn(
      'bg-background relative flex w-full flex-1 flex-col',
      'md:peer-db-[variant=inset]:m-2 md:peer-db-[variant=inset]:ml-0 md:peer-db-[variant=inset]:rounded-xl md:peer-db-[variant=inset]:shadow-sm md:peer-db-[variant=inset]:peer-db-[state=collapsed]:ml-2',
      props.class,
    )"
  >
    <slot />
  </main>
</template>
