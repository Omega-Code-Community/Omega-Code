<script setup lang="ts">
import type { HTMLAttributes } from "vue"
import { cn } from "@/lib/utils"
import { useSidebar } from "./utils"

const props = defineProps<{
  class?: HTMLAttributes["class"]
}>()

const { toggleSidebar } = useSidebar()
</script>

<template>
  <button
    data-sidebar="rail"
    data-slot="sidebar-rail"
    aria-label="Toggle Sidebar"
    :tabindex="-1"
    title="Toggle Sidebar"
    :class="cn(
      'hover:after:bg-sidebar-border absolute inset-y-0 z-20 hidden w-4 -translate-x-1/2 transition-all ease-linear group-db-[side=left]:-right-4 group-db-[side=right]:left-0 after:absolute after:inset-y-0 after:left-1/2 after:w-[2px] sm:flex',
      'in-db-[side=left]:cursor-w-resize in-db-[side=right]:cursor-e-resize',
      '[[db-side=left][db-state=collapsed]_&]:cursor-e-resize [[db-side=right][db-state=collapsed]_&]:cursor-w-resize',
      'hover:group-db-[collapsible=offcanvas]:bg-sidebar group-db-[collapsible=offcanvas]:translate-x-0 group-db-[collapsible=offcanvas]:after:left-full',
      '[[db-side=left][db-collapsible=offcanvas]_&]:-right-2',
      '[[db-side=right][db-collapsible=offcanvas]_&]:-left-2',
      props.class,
    )"
    @click="toggleSidebar"
  >
    <slot />
  </button>
</template>
