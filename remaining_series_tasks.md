# Task List: Remaining Vaccine Groups & Series Implementation

This task list tracks the remaining vaccine groups and series from the legacy Drools-based Java ICE engine that need to be ported to the high-performance Rust forecaster PoC.

To implement a group, follow the guidelines in the [Onboarding & Implementation Guide](file:///home/jason/.gemini/antigravity/brain/9400bc4d-c837-43ac-95b0-8fb76e962685/agent_onboarding_guide.md).

---

## 1. DTaP / Tdap / Td / DTP Series
- [x] Implement `DTP3DoseSeries.yml` (Adult series)
- [x] Implement `DTP5DoseSeries.yml` (Child series)
- [x] Implement same-day priority sorting & custom evaluation logic

## 2. Hepatitis B (`HEP_B`)
- [x] Implement `HepB3DoseChildAdolescentSeries.yml`
- [x] Implement `HepB4DoseChildAdolescentSeries.yml`
- [x] Implement `HepB3DoseTwinrixSeries.yml`
- [x] Implement `HepB4DoseTwinrixSeries.yml`
- [x] Implement `HepBAdult2DoseSeries.yml` (Heplisav-B CVX 189)
- [x] Implement `HepBAdult3DoseSeries.yml`
- [x] Implement custom switch/selection logic between Twinrix, child/adolescent, and adult series

## 3. HPV (Human Papillomavirus)
- [x] Implement `HPV2DoseSeries.yml`
- [x] Implement `HPV3DoseSeries.yml`
- [x] Implement custom series selection based on age of initiation (Dose 1 age < 15y vs >= 15y)

## 4. Hib (Haemophilus influenzae type b)
- [x] Implement `Hib4DoseSeries.yml` (ActHIB, Hiberix, Pentacel, etc.)
- [x] Implement `HibOMPSeries.yml` (PedvaxHIB 3-dose series)
- [x] Implement custom series switching depending on whether OMP or non-OMP vaccines are administered


## 5. Pneumococcal (PCV / PPSV)
- [x] Implement `PneumococcalSeries.yml` (PCV13, PCV15, PCV20, PPSV23)
- [x] Implement complex risk-group and sequence-based evaluation rules

## 6. Meningococcal Conjugate (MCV4)
- [ ] Implement `MCV42DoseSeries.yml` (Menactra, Menveo, MenQuadfi)

## 7. Meningococcal B (MenB)
- [ ] Implement `MenB4C2DoseSeries.yml` & `MenB4C3DoseSeries.yml` (Bexsero)
- [ ] Implement `MenBFHbp2DoseSeries.yml` & `MenBFHbp3DoseSeries.yml` (Trumenba)
- [ ] Enforce product-specific brand consistency rules (do not mix Bexsero and Trumenba)

## 8. Rotavirus
- [ ] Implement `Rotavirus2DoseSeries.yml` (Rotarix)
- [ ] Implement `Rotavirus3DoseSeries.yml` (RotaTeq)
- [ ] Enforce strict age clamps (max age of first dose 14 weeks + 6 days, max age of final dose 8 months + 0 days)

## 9. Seasonal Influenza
- [ ] Implement `Influenza1DoseSeries.yml`
- [ ] Implement `Influenza2DoseSeries.yml`
- [ ] Implement `Influenza2DoseDefaultSeries.yml`
- [ ] Integrate annual influenza season-boundary logic and age-based dose count requirements (2 doses for vaccine-naive young children)

## 10. COVID-19 (Standard & Season-Specific)
- [ ] Implement Pfizer, Moderna, Novavax, Janssen, AstraZeneca, and other international vaccine schedules
- [ ] Implement the September 2023 season schedules (age < 5y vs >= 5y)
- [ ] Implement the August 2025 season schedules (age < 2y, 2y-64y, >= 65y)

## 11. Mpox
- [ ] Implement `Mpox1DoseSeries.yml` & `Mpox2DoseSeries.yml`

## 12. RSV (Respiratory Syncytial Virus)
- [ ] Implement `RSVAdultSeries.yml` (Arexvy, Abrysvo)
- [ ] Implement `RSVInfantSeries.yml` (Beyfortus/Nirsevimab, Synagis/Palivizumab)

## 13. JEV (Japanese Encephalitis)
- [ ] Implement `JEVCRisk2DoseSeries.yml` & `JEVCRisk2DoseAcceleratedSeries.yml` (Ixiaro)

## 14. Cholera
- [ ] Implement `Cholera1DoseRiskSeries.yml` (Vaxchora)

## 15. Typhoid
- [ ] Implement `TyphoidRiskSeries.yml` (Typhim Vi, Vivotif)

## 16. Yellow Fever
- [ ] Implement `YellowFeverRiskSeries.yml` (YF-Vax)

## 17. Historical H1N1 Influenza
- [ ] Implement `H1N11DoseSeries.yml` & `H1N12DoseSeries.yml`
