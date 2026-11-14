# Pre-submit checklist — NASA Space Apps 2026

Deadline: **Sun 2026-11-15 23:59 local time** (team's Local Event; Universal
Event = Team Owner's local time). Internal target: 18:00. Late saves are
ineligible; the **last saved** submission is what gets judged. Resubmitting
is allowed until the deadline — submit early, then improve.

## Required form fields (all filled, all English)

- [ ] Project Name — text in [`project-page.md`](project-page.md)
- [ ] High-Level Project Summary — ≤ 120 words (current draft: 101)
- [ ] Link to Project Demo — public, no login
- [ ] Link to Final Project — public, no login/permission/registration
- [ ] Detailed Project Description
- [ ] Use of Super Intelligence (SI) — filled, matches `SI_USAGE.md`
- [ ] NASA Data — at least one NASA resource listed (IRSA SPHEREx) —
      required for Global Awards
- [ ] Space Agency Partner Data and Other References — complete; **every**
      external resource (code, text, images, even open source) is listed

## The two mandatory checkboxes

- [ ] Agree to Participant Terms / submission guide
- [ ] Confirm originality **and** that every external resource is listed in
      References — only tick after the References list is complete
      (`docs/submission/list-deps.sh` output is already pasted)

## Public links actually work

- [ ] Demo link opens in a **private/incognito window** (no session, no
      login prompt)
- [ ] Final project link opens in a private window
- [ ] Final project link works on a phone-sized viewport (375 px)
- [ ] `?embed=1` URL tested inside an iframe

## Hosting / network readiness

- [ ] Deployment is live and stable (`GET /api/v1/health` returns `ok`)
- [ ] `WARM_TARGETS` set for the five challenge targets, cache warm
- [ ] **FortiGuard classification requested** for the hosted URL at
      https://www.fortiguard.com/webfilter (suggest "Education" or
      "Information Technology"; note it is a NASA Space Apps hackathon
      project — reviews take ~24 h, do this as soon as the URL exists)
- [ ] FortiGuard category re-checked before submitting

## Demo limits

- [ ] Video ≤ 30 s **or** slide deck ≤ 7 slides **including title/intro** —
      exceeding either = ineligible for Global Judging
- [ ] Slides show team name, member names and the chosen challenge
- [ ] Demo shows the project image, the UX, and how NASA/partner data make
      it possible
- [ ] Any non-English speech has English subtitles
- [ ] Form images are JPG/PNG/GIF ≤ 10 MB (no video/doc uploads to the form)

## Content rules

- [ ] Everything in English
- [ ] **No NASA logos, flags or mission identifiers** in any generated
      content (screenshots, slides, video, site)
- [ ] No photo/video/voice/likeness of anyone under 18
- [ ] No PII; no content we lack rights to
- [ ] SI use described in the SI field; code/docs acknowledged as
      SI-assisted (`SI_USAGE.md`, commit trailers); no SI-generated
      images/audio/video used (so no watermarks needed)
- [ ] Repo license file present (Apache-2.0); original content freely
      licensed
- [ ] All `TODO(team):` and `TODO(measured):` markers in
      `docs/submission/*.md` resolved — none left in pasted text

## Final pass

- [ ] Last saved version reviewed end-to-end by a second team member
- [ ] Submitted **before** the deadline (aim for the 18:00 internal target)
- [ ] Confirmation email/screenshot saved by the team
