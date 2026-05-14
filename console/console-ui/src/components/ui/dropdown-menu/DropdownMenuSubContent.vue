<script setup lang="ts">
import type { DropdownMenuSubContentEmits, DropdownMenuSubContentProps } from "reka-ui"
import type { HTMLAttributes } from "vue"
import { reactiveOmit } from "@vueuse/core"
import {
  DropdownMenuSubContent,
  useForwardPropsEmits,
} from "reka-ui"
import { cn } from "@/lib/utils"

const props = defineProps<DropdownMenuSubContentProps & { class?: HTMLAttributes["class"] }>()
const emits = defineEmits<DropdownMenuSubContentEmits>()

const delegatedProps = reactiveOmit(props, "class")

const forwarded = useForwardPropsEmits(delegatedProps, emits)
</script>

<template>
  <DropdownMenuSubContent
    data-slot="dropdown-menu-sub-content"
    v-bind="forwarded"
    :class="cn('bg-popover text-popover-foreground db-[state=open]:animate-in db-[state=closed]:animate-out db-[state=closed]:fade-out-0 db-[state=open]:fade-in-0 db-[state=closed]:zoom-out-95 db-[state=open]:zoom-in-95 db-[side=bottom]:slide-in-from-top-2 db-[side=left]:slide-in-from-right-2 db-[side=right]:slide-in-from-left-2 db-[side=top]:slide-in-from-bottom-2 z-50 min-w-[8rem] origin-(--reka-dropdown-menu-content-transform-origin) overflow-hidden rounded-md border p-1 shadow-lg', props.class)"
  >
    <slot />
  </DropdownMenuSubContent>
</template>
