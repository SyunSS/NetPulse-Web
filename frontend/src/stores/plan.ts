import { defineStore } from 'pinia'
import { ref } from 'vue'
import { planApi, type PlanWithItems, type TaskPlanRun } from '@/api/plan'

export const usePlanStore = defineStore('plan', () => {
  const plans = ref<PlanWithItems[]>([])
  const total = ref(0)
  const loading = ref(false)
  const currentPlan = ref<PlanWithItems | null>(null)
  const planRuns = ref<TaskPlanRun[]>([])

  async function fetchPlans(page = 1, size = 20) {
    loading.value = true
    try {
      const res = await planApi.list(page, size)
      plans.value = res.data.plans
      total.value = res.data.total
    } finally {
      loading.value = false
    }
  }

  async function fetchPlan(planId: string) {
    const res = await planApi.get(planId)
    currentPlan.value = res.data
    return res.data
  }

  async function createPlan(data: any) {
    const res = await planApi.create(data)
    return res.data
  }

  async function updatePlan(planId: string, data: any) {
    const res = await planApi.update(planId, data)
    return res.data
  }

  async function deletePlan(planId: string) {
    await planApi.delete(planId)
  }

  async function runPlan(planId: string) {
    const res = await planApi.run(planId)
    return res.data
  }

  async function fetchPlanRuns(planId: string, params?: { start?: string; end?: string }) {
    const res = await planApi.runs(planId, params)
    planRuns.value = res.data
    return res.data
  }

  async function deleteRun(planId: string, runId: string, force = false) {
    await planApi.deleteRun(planId, runId, force)
  }

  return {
    plans,
    total,
    loading,
    currentPlan,
    planRuns,
    fetchPlans,
    fetchPlan,
    createPlan,
    updatePlan,
    deletePlan,
    runPlan,
    fetchPlanRuns,
    deleteRun,
  }
})
