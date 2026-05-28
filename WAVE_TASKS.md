# StellarDisburse — Drips Wave 5 Task Backlog

Welcome to the StellarDisburse Drips Wave task backlog! Earn points by improving our open disbursement infrastructure.

---

## 🟢 Trivial Tasks (100 Points)

### SD-T01: CSV Export Capabilities for Bulk Disbursements
* **Description:** Add client-side CSV parsing and downloading capabilities to export recent disbursement histories.
* **Complexity:** Trivial (100 pts)
* **Status:** Open 🚀
* **Files to Edit:**
  * `stellardisburse-portal/src/pages/Disbursements.tsx`
  * `stellardisburse-portal/src/utils/csv.ts`

---

## 🟡 Medium Tasks (150 Points)

### SD-M01: Disbursements History Performance Charts
* **Description:** Integrate Recharts graphs to visualize total monthly disbursement volumes, successful transactions, and pending batches.
* **Complexity:** Medium (150 pts)
* **Status:** Open 🚀
* **Files to Edit:**
  * `stellardisburse-portal/src/pages/Dashboard.tsx`
  * `stellardisburse-portal/src/components/VolumeChart.tsx`
* **Acceptance Criteria:**
  * Renders beautiful responsive area charts with interactive tooltips.

---

## 🔴 High Tasks (200 Points)

### SD-H01: Implement Resilient Batch Retry Hooks
* **Description:** Add auto-retry queues to process failed disbursements on network congestion, utilizing exponential backoff.
* **Complexity:** High (200 pts)
* **Status:** Open 🚀
* **Files to Edit:**
  * `stellardisburse-sdk/src/client/DisbursementClient.ts`
  * `stellardisburse-sdk/src/queue/RetryQueue.ts`
* **Acceptance Criteria:**
  * Retries failed transactions up to 5 times, correctly logging sequence number errors.
