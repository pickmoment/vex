# harness-do

slug = `$ARGUMENTS` (없으면 사용자에게 slug 요청)

**control_dir** = `.claude/tasks/<slug>`
**state_file** = `.claude/tasks/<slug>/_state.json`

## 절차

1. `_state.json` 읽기 → `layout` 확인 (`artifacts_dir` 필드)
2. plan.md를 읽어 스테이지 구조 파악 (`[스테이지]` 정보 또는 직접 파악)
   - **이 컨텍스트에 plan.md 전체를 적재하지 말 것** — 스테이지 목록과 병렬 여부만 파악
3. 스테이지 구조에 따라 수행 에이전트 호출:
   - **병렬 스테이지**: 단일 메시지에서 Agent 동시 호출
   - **순차 스테이지**: 순서대로 Agent 호출

각 수행 에이전트 프롬프트:

```
Agent(
  prompt="""
plan.md 경로: .claude/tasks/<slug>/plan.md
담당 스테이지: S<N>
artifacts 저장 경로: <layout.artifacts_dir>
exec-log.md 경로: .claude/tasks/<slug>/exec-log.md

plan.md를 직접 읽고 S<N> 실행.
산출물을 artifacts 저장 경로에 저장하고 exec-log.md에 append로 기록.

보고 형식:
[결과] 성공 | 실패
[저장 경로] <파일 경로>
[요약] 한 줄
[실패 사유] (실패 시)
"""
)
```

4. 모든 스테이지 완료 후 즉시: `_state.json` → `phase: "check"` 갱신
5. 결과를 사용자에게 보고
