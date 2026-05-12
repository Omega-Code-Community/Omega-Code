<script setup lang="ts">
import {
  IconCamera,
  IconDashboard,
  IconFileAi,
  IconFileDescription,
  IconHelp,
  IconInnerShadowTop,
  IconSettings,
} from "@tabler/icons-vue"

import {
  Bot,
  Cloudy,
  Database,
  DatabaseZap,
  HatGlasses,
  LandPlot,
  MessageSquareMore,
  MessageSquareText,
  MessagesSquare,
  Store,
  Target,
  ToolCase,
} from "lucide-vue-next"
import NavMain from '@/components/NavMain.vue'
import NavSecondary from '@/components/NavSecondary.vue'
import {
  Sidebar,
  SidebarContent,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from '@/components/ui/sidebar'
import { useI18n } from 'vue-i18n'
const { t } = useI18n()



const data = {
  user: {
    name: "shadcn",
    email: "m@example.com",
    avatar: "/avatars/shadcn.jpg",
  },
  navMain: [
    {
      title: t("siderbar.dashboard"),
      url: "#",
      icon: IconDashboard,
    },
    {
      title: t("siderbar.aiProvider"),
      url: "#",
      icon: Store,
    },
    {
      title: t("siderbar.llm"),
      url: "#",
      icon: Bot,
    },
    {
      title: t("siderbar.agent"),
      url: "#",
      icon: HatGlasses,
    },
    {
      title: t("siderbar.prompt"),
      url: "#",
      icon: MessageSquareText,
    },
    {
      title: t("siderbar.agentArena"),
      url: "#",
      icon: LandPlot,
    },
    {
      title: t("siderbar.chatSession"),
      url: "#",
      icon: MessagesSquare,
    },
    {
      title: t("siderbar.chatMessage"),
      url: "#",
      icon: MessageSquareMore,
    },
    {
      title: t("siderbar.memory"),
      url: "#",
      icon: DatabaseZap,
    },
    {
      title: t("siderbar.mcpProvider"),
      url: "#",
      icon: Cloudy,
    },
    {
      title: t("siderbar.mcpTools"),
      url: "#",
      icon: ToolCase,
    },
    {
      title: t("siderbar.mcpToolBinding"),
      url: "#",
      icon: Target,
    },
    {
      title: t("siderbar.faissCpu"),
      url: "#",
      icon: Database,
    },
  ],
  navClouds: [
    {
      title: "Capture",
      icon: IconCamera,
      isActive: true,
      url: "#",
      items: [
        {
          title: "Active Proposals",
          url: "#",
        },
        {
          title: "Archived",
          url: "#",
        },
      ],
    },
    {
      title: "Proposal",
      icon: IconFileDescription,
      url: "#",
      items: [
        {
          title: "Active Proposals",
          url: "#",
        },
        {
          title: "Archived",
          url: "#",
        },
      ],
    },
    {
      title: "Prompts",
      icon: IconFileAi,
      url: "#",
      items: [
        {
          title: "Active Proposals",
          url: "#",
        },
        {
          title: "Archived",
          url: "#",
        },
      ],
    },
  ],
  navSecondary: [
    {
      title: t("siderbar.settings"),
      url: "#",
      icon: IconSettings,
    },
    {
      title: t("siderbar.getHelp"),
      url: "#",
      icon: IconHelp,
    }
  ]
}
</script>

<template>
  <Sidebar collapsible="offcanvas">
    <SidebarHeader>
      <SidebarMenu>
        <SidebarMenuItem>
          <SidebarMenuButton
            as-child
            class="data-[slot=sidebar-menu-button]:!p-1.5"
          >
            <a href="#">
              <IconInnerShadowTop class="!size-5" />
              <span class="text-base font-semibold">Omega Code Console.</span>
            </a>
          </SidebarMenuButton>
        </SidebarMenuItem>
      </SidebarMenu>
    </SidebarHeader>
    <SidebarContent>
      <NavMain :items="data.navMain" />
      <NavSecondary :items="data.navSecondary" class="mt-auto" />
    </SidebarContent>
  </Sidebar>
</template>
