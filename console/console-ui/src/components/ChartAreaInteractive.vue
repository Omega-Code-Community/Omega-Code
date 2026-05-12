<template>
  <div class="">
    <!-- 顶部指标概览 -->
    <div class="*:data-[slot=card]:from-primary/5 *:data-[slot=card]:to-card dark:*:data-[slot=card]:bg-card grid grid-cols-1 gap-4 *:data-[slot=card]:bg-gradient-to-t *:data-[slot=card]:shadow-xs @xl/main:grid-cols-2 @5xl/main:grid-cols-4">
      <Card v-for="(item, index) in cardData" :key="index" class="@container/card">
        <CardHeader>
          <CardDescription>{{ item.title }}</CardDescription>
          <CardTitle class="text-2xl font-semibold tabular-nums @[250px]/card:text-3xl">
            {{ item.value }}
          </CardTitle>
          <CardAction>
            <Badge :variant="item.trend.up ? 'default' : 'destructive'" class="flex items-center gap-1">
              <IconTrendingUp v-if="item.trend.up" class="size-3" />
              <IconTrendingDown v-else class="size-3" />
              {{ item.trend.value }}
            </Badge>
          </CardAction>
        </CardHeader>
        <CardFooter class="flex-col items-start gap-1.5 text-sm">
          <div class="line-clamp-1 flex gap-2 font-medium">
            {{ item.insight }} 
            <IconRobot v-if="index === 0" class="size-4 text-primary" />
            <IconCloud v-if="index === 1" class="size-4 text-secondary" />
            <IconMessage2 v-if="index === 2" class="size-4 text-accent" />
            <IconBolt v-if="index === 3" class="size-4 text-success" />
          </div>
          <div class="text-muted-foreground">
            {{ item.description }}
          </div>
        </CardFooter>
      </Card>
    </div>

    <!-- 性能指标 -->
    <div class="*:data-[slot=card]:from-primary/5 *:data-[slot=card]:to-card dark:*:data-[slot=card]:bg-card grid grid-cols-1 gap-4 *:data-[slot=card]:bg-gradient-to-t *:data-[slot=card]:shadow-xs @xl/main:grid-cols-2 @5xl/main:grid-cols-4">
      <Card v-for="(item, index) in performanceData" :key="index" class="@container/card">
        <CardHeader>
          <CardDescription>{{ item.title }}</CardDescription>
          <CardTitle class="text-2xl font-semibold tabular-nums @[250px]/card:text-3xl">
            {{ item.value }}
          </CardTitle>
          <CardAction>
            <Badge :variant="item.trend.up ? 'success' : 'undefined'" class="flex items-center gap-1">
              <IconTrendingUp v-if="item.trend.up" class="size-3" />
              <IconTrendingDown v-else class="size-3" />
              {{ item.trend.value }}
            </Badge>
          </CardAction>
        </CardHeader>
        <CardFooter class="flex-col items-start gap-1.5 text-sm">
          <div class="line-clamp-1 flex gap-2 font-medium">
            {{ item.insight }}
            <IconClock v-if="index === 0" class="size-4 text-warning" />
            <IconChartBar v-if="index === 1" class="size-4 text-info" />
            <IconStar v-if="index === 3" class="size-4 text-yellow-400" />
          </div>
          <div class="text-muted-foreground">
            {{ item.description }}
          </div>
        </CardFooter>
      </Card>
    </div>

    <!-- 业务趋势图表 -->
    <Card class="pt-0">
      <CardHeader class="flex items-center gap-2 space-y-0 border-b py-5 sm:flex-row">
        <div class="grid flex-1 gap-1">
          <CardTitle>AI Agent Platform Analytics</CardTitle>
          <CardDescription>
            Key metrics and trends for your intelligent agent ecosystem
          </CardDescription>
        </div>
        <Select v-model="timeRange">
          <SelectTrigger
            class="hidden w-[160px] rounded-lg sm:ml-auto sm:flex"
            aria-label="Select time range"
          >
            <SelectValue placeholder="Last 30 days" />
          </SelectTrigger>
          <SelectContent class="rounded-xl">
            <SelectItem value="7d" class="rounded-lg">
              Last 7 days
            </SelectItem>
            <SelectItem value="30d" class="rounded-lg">
              Last 30 days
            </SelectItem>
            <SelectItem value="90d" class="rounded-lg">
              Last 90 days
            </SelectItem>
          </SelectContent>
        </Select>
      </CardHeader>
      <CardContent class="px-2 pt-4 sm:px-6 sm:pt-6 pb-4">
        <ChartContainer :config="chartConfig" class="aspect-auto h-[350px] w-full" :cursor="false">
          <VisXYContainer
            :data="filterRange"
            :svg-defs="svgDefs"
            :margin="{ left: -40, bottom: -20 }"
            :y-domain="[0, 3000]"
          >
            <VisArea
              :x="(d: ChartData) => d.date"
              :y="[(d: ChartData) => d.sessions]"
              :color="'url(#fillSessions)'"
              :opacity="0.6"
            />
            <VisLine
              :x="(d: ChartData) => d.date"
              :y="[(d: ChartData) => d.sessions]"
              :color="chartConfig.sessions.color"
              :line-width="2"
            />
            <VisAxis
              type="x"
              :x="(d: ChartData) => d.date"
              :tick-line="false"
              :domain-line="false"
              :grid-line="false"
              :num-ticks="6"
              :tick-format="(d: number) => {
                const date = new Date(d)
                return date.toLocaleDateString('en-US', {
                  month: 'short',
                  day: 'numeric',
                })
              }"
            />
            <VisAxis
              type="y"
              :num-ticks="4"
              :tick-line="false"
              :domain-line="false"
            />
            <ChartTooltip />
            <ChartCrosshair
              :template="componentToString(chartConfig, ChartTooltipContent, {
                labelFormatter: (d) => {
                  return new Date(d).toLocaleDateString('en-US', {
                    month: 'short',
                    day: 'numeric',
                    year: 'numeric'
                  })
                },
                valueFormatter: (value, name) => {
                  if (name === 'tokens') {
                    return (value / 1000000).toFixed(1) + 'M'
                  }
                  if (name === 'satisfaction') {
                    return value.toFixed(1) + '/5'
                  }
                  return value.toString()
                }
              })"
            />
          </VisXYContainer>
          <ChartLegendContent class="mt-4" />
        </ChartContainer>
      </CardContent>
    </Card>

    <!-- 业务细分指标 -->
    <div class="grid grid-cols-1 gap-4 @xl/main:grid-cols-2">
      <Card>
        <CardHeader>
          <CardTitle>Agent Performance Matrix</CardTitle>
          <CardDescription>Top performing agents by usage and satisfaction</CardDescription>
        </CardHeader>
        <CardContent>
          <div class="space-y-4">
            <div v-for="(agent, index) in topAgents" :key="index" class="flex items-center justify-between p-3 hover:bg-muted/50 rounded-lg transition-colors">
              <div class="flex items-center gap-3">
                <div class="size-10 rounded-full bg-primary/10 flex items-center justify-center">
                  <IconRobot class="size-5 text-primary" />
                </div>
                <div>
                  <div class="font-medium">{{ agent.name }}</div>
                  <div class="text-sm text-muted-foreground">{{ agent.category }}</div>
                </div>
              </div>
              <div class="flex items-center gap-4">
                <div class="text-right">
                  <div class="font-medium">{{ agent.usage }} sessions</div>
                  <div class="text-sm text-muted-foreground">This week</div>
                </div>
                <Badge :variant="agent.rating >= 4.5 ? 'success' : 'warning'" class="px-3 py-1">
                  {{ agent.rating }}/5
                </Badge>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Model Distribution</CardTitle>
          <CardDescription>LLM usage across different vendors and models</CardDescription>
        </CardHeader>
        <CardContent>
          <div class="space-y-4">
            <div v-for="(model, index) in modelDistribution" :key="index" class="flex items-center justify-between p-3 hover:bg-muted/50 rounded-lg transition-colors">
              <div class="flex items-center gap-3">
                <div class="size-10 rounded-full bg-secondary/10 flex items-center justify-center">
                  <span class="text-sm font-medium">{{ model.initials }}</span>
                </div>
                <div>
                  <div class="font-medium">{{ model.name }}</div>
                  <div class="text-sm text-muted-foreground">{{ model.vendor }}</div>
                </div>
              </div>
              <div class="flex items-center gap-4">
                <div class="w-24 bg-secondary/20 rounded-full h-2 overflow-hidden">
                  <div class="h-full bg-secondary rounded-full" :style="{ width: model.percentage + '%' }"></div>
                </div>
                <span class="font-medium w-12 text-right">{{ model.percentage }}%</span>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>

    <!-- 系统健康状态 -->
    <Card>
      <CardHeader>
        <CardTitle>System Health & Performance</CardTitle>
        <CardDescription>Real-time monitoring of platform infrastructure</CardDescription>
      </CardHeader>
      <CardContent>
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="space-y-3">
            <div class="flex items-center justify-between">
              <span class="font-medium">API Health</span>
              <Badge variant="success" class="px-3 py-1">100% Healthy</Badge>
            </div>
            <div class="space-y-2">
              <div v-for="(service, index) in systemServices" :key="index" class="flex items-center justify-between">
                <span class="text-muted-foreground">{{ service.name }}</span>
                <Badge :variant="service.status === 'healthy' ? 'success' : 'warning'" class="px-2 py-1">
                  {{ service.status }}
                </Badge>
              </div>
            </div>
          </div>
          
          <div class="space-y-3">
            <div class="flex items-center justify-between">
              <span class="font-medium">Resource Utilization</span>
              <Badge variant="warning" class="px-3 py-1">Monitor</Badge>
            </div>
            <div class="space-y-2">
              <div v-for="(resource, index) in resources" :key="index" class="flex items-center justify-between">
                <span class="text-muted-foreground">{{ resource.name }}</span>
                <div class="w-32 bg-secondary/20 rounded-full h-2 overflow-hidden">
                  <div class="h-full bg-secondary rounded-full" :style="{ width: resource.usage + '%' }"></div>
                </div>
                <span class="ml-2 text-sm font-medium">{{ resource.usage }}%</span>
              </div>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  </div>
</template>
<script setup lang="ts">
import { IconUsers, IconRobot, IconMessage2, IconCloud, IconChartBar, IconBolt, IconStar, IconClock } from "@tabler/icons-vue"
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
const { t } = useI18n()

import { Badge } from '@/components/ui/badge'
import {
  Card,
  CardAction,
  CardDescription,
  CardContent,
  CardFooter,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import {
  ChartContainer,
  ChartCrosshair,
  ChartLegendContent,
  ChartTooltip,
  ChartTooltipContent,
  componentToString,
} from '@/components/ui/chart'
import { VisArea, VisAxis, VisLine, VisXYContainer } from "@unovis/vue"
// 模拟业务数据
const topAgents = [
  { name: "Code Assistant", category: "Development", usage: 487, rating: 4.9 },
  { name: "Data Analyst", category: "Analytics", usage: 356, rating: 4.8 },
  { name: "Customer Support", category: "Service", usage: 298, rating: 4.7 },
  { name: "Content Creator", category: "Marketing", usage: 245, rating: 4.6 },
  { name: "Security Advisor", category: "Security", usage: 189, rating: 4.8 }
]

const modelDistribution = [
  { name: "DeepSeek v4", vendor: "DeepSeek", initials: "DS", percentage: 38 },
  { name: "Gimini 3.5", vendor: "Google", initials: "GM", percentage: 25 },
  { name: "ChatGPT 5", vendor: "OpenAI", initials: "GPT", percentage: 22 },
  { name: "Claude 3", vendor: "Anthropic", initials: "CL", percentage: 15 }
]

const systemServices = [
  { name: "LLM Gateway", status: "healthy" },
  { name: "MCP Integration", status: "healthy" },
  { name: "Memory Service", status: "healthy" },
  { name: "Agent Orchestrator", status: "healthy" }
]

const resources = [
  { name: "CPU Utilization", usage: 65 },
  { name: "Memory Usage", usage: 72 },
  { name: "Database Load", usage: 48 },
  { name: "API Throughput", usage: 85 }
]
// 模拟业务数据
const cardData = [
  {
    title: t("dashboard.activeAgent"),
    value: "42",
    description: t("dashboard.totalActiveIntelligentAgents"),
    trend: { up: true, value: "+15.2%" },
    insight: t("dashboard.growingAgentEcosystem")
  },
  {
    title: t("dashboard.llmModel"),
    value: "24",
    description: t("dashboard.availableAiModels"),
    trend: { up: true, value: "+8.3%" },
    insight: t("dashboard.multiVendorSupportExpandingEcosystem")
  },
  {
    title: t("dashboard.DailySessions"),
    value: "1,872",
    description: t("dashboard.activeChatSessions"),
    trend: { up: false, value: "-3.1%" },
    insight: t("dashboard.weekendUsagePatterns")
  },
  {
    title: t("dashboard.mcpTools"),
    value: "86",
    description: t("dashboard.connectedToolsCapabilities"),
    trend: { up: true, value: "+22.5%" },
    insight: t("dashboard.ecoEcosystemGrowthAccelerating")
  }
]

const performanceData = [
  {
    title: t("dashboard.avgResponseTime"),
    value: "1.2s",
    description: t("dashboard.modelInferenceLatency"),
    trend: { up: false, value: "-18.7%" },
    insight: t("dashboard.optimizationImprovingSpeed")
  },
  {
    title: t("dashboard.tokenUsage"),
    value: "28.5M",
    description: t("dashboard.monthlyTokenConsumption"),
    trend: { up: true, value: "+34.2%" },
    insight: t("dashboard.scalingWithUserBase")
  },
  {
    title: t("dashboard.successRate"),
    value: "98.7%",
    description: t("dashboard.apiCallSuccessRate"),
    trend: { up: true, value: "+1.2%" },
    insight: t("dashboard.stableInfrastructure")
  },
  {
    title: t("dashboard.userSatisfaction"),
    value: "4.8/5",
    description: t("dashboard.avgUserRating"),
    trend: { up: true, value: "+0.3" },
    insight: t("dashboard.positiveUserFeedback")
  }
]

// 图表数据
const chartData = [
  { date: new Date("2024-04-01"), sessions: 850, tokens: 12500000, agents: 32, satisfaction: 4.6 },
  { date: new Date("2024-04-08"), sessions: 920, tokens: 13800000, agents: 34, satisfaction: 4.5 },
  { date: new Date("2024-04-15"), sessions: 1050, tokens: 15200000, agents: 36, satisfaction: 4.7 },
  { date: new Date("2024-04-22"), sessions: 1180, tokens: 16800000, agents: 38, satisfaction: 4.6 },
  { date: new Date("2024-04-29"), sessions: 1250, tokens: 18500000, agents: 40, satisfaction: 4.7 },
  { date: new Date("2024-05-06"), sessions: 1320, tokens: 20100000, agents: 41, satisfaction: 4.8 },
  { date: new Date("2024-05-13"), sessions: 1450, tokens: 22500000, agents: 42, satisfaction: 4.7 },
  { date: new Date("2024-05-20"), sessions: 1580, tokens: 24800000, agents: 43, satisfaction: 4.8 },
  { date: new Date("2024-05-27"), sessions: 1650, tokens: 26200000, agents: 44, satisfaction: 4.9 },
  { date: new Date("2024-06-03"), sessions: 1720, tokens: 27500000, agents: 45, satisfaction: 4.8 },
  { date: new Date("2024-06-10"), sessions: 1780, tokens: 28100000, agents: 46, satisfaction: 4.9 },
  { date: new Date("2024-06-17"), sessions: 1820, tokens: 28500000, agents: 47, satisfaction: 5.0 },
  { date: new Date("2024-06-24"), sessions: 1872, tokens: 28500000, agents: 48, satisfaction: 4.8 }
]

type ChartData = typeof chartData[number]

const chartConfig = {
  sessions: {
    label: "Sessions",
    color: "hsl(var(--primary))",
  },
  tokens: {
    label: "Tokens (M)",
    color: "hsl(var(--secondary))",
  },
  agents: {
    label: "Active Agents",
    color: "hsl(var(--accent))",
  },
  satisfaction: {
    label: "Satisfaction",
    color: "hsl(var(--success))",
  }
}

const svgDefs = `
  <linearGradient id="fillSessions" x1="0" y1="0" x2="0" y2="1">
    <stop offset="5%" stop-color="var(--color-sessions)" stop-opacity="0.8" />
    <stop offset="95%" stop-color="var(--color-sessions)" stop-opacity="0.1" />
  </linearGradient>
  <linearGradient id="fillTokens" x1="0" y1="0" x2="0" y2="1">
    <stop offset="5%" stop-color="var(--color-tokens)" stop-opacity="0.8" />
    <stop offset="95%" stop-color="var(--color-tokens)" stop-opacity="0.1" />
  </linearGradient>
  <linearGradient id="fillAgents" x1="0" y1="0" x2="0" y2="1">
    <stop offset="5%" stop-color="var(--color-agents)" stop-opacity="0.8" />
    <stop offset="95%" stop-color="var(--color-agents)" stop-opacity="0.1" />
  </linearGradient>
  <linearGradient id="fillSatisfaction" x1="0" y1="0" x2="0" y2="1">
    <stop offset="5%" stop-color="var(--color-satisfaction)" stop-opacity="0.8" />
    <stop offset="95%" stop-color="var(--color-satisfaction)" stop-opacity="0.1" />
  </linearGradient>
`

const timeRange = ref("30d")
const filterRange = computed(() => {
  const data = [...chartData].sort((a, b) => a.date.getTime() - b.date.getTime())
  const endDate = data[data.length - 1].date
  let startDate = new Date(endDate)
  
  if (timeRange.value === "7d") {
    startDate.setDate(endDate.getDate() - 7)
  } else if (timeRange.value === "30d") {
    startDate.setDate(endDate.getDate() - 30)
  } else if (timeRange.value === "90d") {
    startDate.setDate(endDate.getDate() - 90)
  }
  
  return data.filter(item => item.date >= startDate)
})
</script>

