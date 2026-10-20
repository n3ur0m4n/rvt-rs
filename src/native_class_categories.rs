//! What the API reports for an element because of its native class: the
//! `BuiltInCategory` of the classes whose category is a property of the
//! class rather than of the element, and which classes (and which family
//! categories) expose the IFC export parameters.
//!
//! Revit does not save the category of a system-family element or of most
//! settings elements: the class decides it. The API reports it through
//! `ELEM_CATEGORY_PARAM` (`-1` when the element has no category). This table
//! was measured against API exports of every file of the corpus (Revit
//! 2024-2027, real and synthetic): it lists only the classes whose elements
//! all reported the same value in every file where they occur, with the
//! number of measured elements. A class absent here gives no category.
//!
//! Classes whose category varies by element (`FamilyInstance`,
//! `FamilySymbol`, `FloorAttributes`, `Floor`, dimensions, tags, curves,
//! imports, groups, `SpotElevation`, `Viewer`, ...) are deliberately absent;
//! `native_parameter_values` resolves the ones it can from the element's own
//! references.

/// `(native class, ELEM_CATEGORY_PARAM value)`.
const CLASS_CATEGORIES: &[(&str, i64)] = &[
    ("AbsDuctType", -2008000),                           // OST_DuctCurves x20
    ("AbsFlexDuctType", -2008020),                       // OST_FlexDuctCurves x10
    ("ActiveGeoLocationTrackingElement", -1),            // None x4
    ("Alignment", -2000262),                             // OST_Constraints x97
    ("AllPlanTopologies", -1),                           // None x24
    ("AllProjectPhases", -1),                            // None x4
    ("AllProjectRevisions", -1),                         // None x4
    ("AnalyticalLinkType", -2009657),                    // OST_LinksAnalytical x4
    ("AnalyticalToPhysicalRelationManager", -1),         // None x4
    ("AngularDim", -2000260),                            // OST_Dimensions x3
    ("AppearanceAssetElem", -1),                         // None x847
    ("AreaMeasureElem", -2003201),                       // OST_AreaSchemes x13
    ("AreaReinforcementType", -2009003),                 // OST_AreaRein x1
    ("AreaReportSettingsElem", -1),                      // None x4
    ("AreaSchemePlanTopologies", -1),                    // None x13
    ("AreaSettingsElem", -1),                            // None x4
    ("AreaTypeElem", -1),                                // None x62
    ("AssemblyCodeTable", -1),                           // None x4
    ("AssemblyTracker", -1),                             // None x4
    ("AutoCamSettingsElem", -1),                         // None x4
    ("AutoJoinTracker", -1),                             // None x4
    ("BaseLevelTracker", -1),                            // None x1
    ("BasicWallType", -2000011),                         // OST_Walls x1164
    ("BrowserOrganization", -1),                         // None x38
    ("BuildingOperatingYearSchedule", -2008121),         // OST_HVAC_Load_Schedules x75
    ("BuildingPadType", -2001263),                       // OST_BuildingPad x3
    ("CableTraySettingsElem", -1),                       // None x4
    ("CableTraySizesElem", -1),                          // None x4
    ("CalloutTag", -1),                                  // None x5
    ("CategoryElem", -1),                                // None x1234
    ("CircuitNamingTypeSetting", -1),                    // None x4
    ("ColorFillSchema", -2000552),                       // OST_ColorFillSchema x51
    ("ColorFillSymbol", -2000550),                       // OST_ColorFillLegends x6
    ("ComponentRepeaterSlotSpecialType", -1),            // None x8
    ("CompoundCeilingType", -2000038),                   // OST_Ceilings x204
    ("ConceptualSurfaceType", -1),                       // None x40
    ("ConduitSettingsElem", -1),                         // None x4
    ("ConduitSizesElem", -1),                            // None x4
    ("ConduitStandardType", -2008144),                   // OST_ConduitStandards x21
    ("ConstructionSet", -2008102),                       // OST_EAConstructions x2
    ("ConstructionSetProject", -1),                      // None x4
    ("ContFootingType", -2001300),                       // OST_StructuralFoundation x6
    ("ContourLabelingAttributes", -1),                   // None x5
    ("ContourSpacingElem", -2001342),                    // OST_TopographyContours x7
    ("CoordinateSystemDisplayElem", -2000977),           // OST_CoordinateSystem x4
    ("CopyWatchProperties", -1),                         // None x3
    ("CorniceAttr", -2000181),                           // OST_Cornices x6
    ("CoverType", -2009014),                             // OST_CoverType x9
    ("CurtaSystemAttr", -2000340),                       // OST_CurtaSystem x5
    ("CurtainRoofAttributes", -2000035),                 // OST_Roofs x4
    ("CustomElement", -1),                               // None x122
    ("CutMarkType", -2000930),                           // OST_StairsCutMarks x6
    ("DBDrawing", -1),                                   // None x197
    ("DBView3d", -2000279),                              // OST_Views x88
    ("DBViewAnonDrafting", -2000279),                    // OST_Views x6
    ("DBViewDrafting", -2003100),                        // None x15
    ("DBViewLegend", -2000279),                          // OST_Views x4
    ("DBViewPlan", -2000279),                            // OST_Views x642
    ("DBViewProject", -1),                               // None x53
    ("DBViewSchedule", -2000573),                        // None x12
    ("DBViewSection", -2000279),                         // OST_Views x525
    ("DBViewType", -1),                                  // None x83
    ("DBViewTypeAreaPlan", -1),                          // None x3
    ("DataStorage", -1),                                 // None x6
    ("DaylightSourceIdSet", -1),                         // None x4
    ("DefaultDivideSettings", -1),                       // None x4
    ("DimensionStyle", -1),                              // None x1050
    ("DivisionRule", -2008190),                          // OST_DivisionRules x68
    ("Dwg2dExportUserSettingsData", -1),                 // None x4
    ("EdgeSlabAttr", -2001392),                          // OST_EdgeSlab x4
    ("ElectricalDemandFactorDefinition", -2008142), // OST_ElectricalDemandFactorDefinitions x71
    ("ElectricalLoadClassification", -2008143),     // OST_ElectricalLoadClassifications x90
    ("ElectricalSetting", -1),                      // None x4
    ("EnergyDataSettings", -1),                     // None x4
    ("ExportPDFSettings", -1),                      // None x2
    ("ExtentElem", -1),                             // None x165
    ("ExternalParamLock", -1),                      // None x4
    ("FabricSheetType", -2009016),                  // OST_FabricReinforcement x1
    ("FabricWireType", -2009016),                   // OST_FabricReinforcement x1
    ("FabricationPartCenterline", -2008193),        // OST_FabricationDuctwork x13
    ("FabricationServiceSettings", -1),             // None x4
    ("FabricationSettings", -1),                    // None x3
    ("FabricationSettingsElement", -1),             // None x4
    ("FamSymSurrogate", -1),                        // None x839
    ("Family", -1),                                 // None x2986
    ("FamilySurrogate", -1),                        // None x520
    ("FasciaAttr", -2001390),                       // OST_Fascia x5
    ("FillPatternElem", -1),                        // None x569
    ("FilledRegion", -2002000),                     // OST_DetailComponents x62
    ("Floor", -2000032),                            // OST_Floors x67
    ("FontElem", -1),                               // None x429
    ("ForgeConnectionTracker", -1),                 // None x1
    ("GCSTracker", -1),                             // None x4
    ("GStyleElem", -1),                             // None x7042
    ("GeoLocation", -2000110),                      // OST_IOS_GeoLocations x14
    ("GeoSite", -2000111),                          // OST_IOS_GeoSite x8
    ("GraphicsCache", -1),                          // None x4
    ("Grid", -2000220),                             // OST_Grids x10
    ("GridAttributes", -2000220),                   // OST_Grids x13
    ("GutterAttr", -2001391),                       // OST_Gutter x4
    ("HVACLoadBuildingTypeElem", -2008120),         // OST_HVAC_Load_Building_Types x134
    ("HVACLoadScheduleElem", -2008121),             // OST_HVAC_Load_Schedules x100
    ("HVACLoadSpaceTypeElem", -2008119),            // OST_HVAC_Load_Space_Types x502
    ("HalftoneUnderlaySettings", -1),               // None x4
    ("HandRailType", -2000947),                     // OST_RailingHandRail x6
    ("HubsTracker", -1),                            // None x4
    ("IFCCategoryTemplate", -1),                    // None x1
    ("ImageHolder", -1),                            // None x1
    ("ImageSymbol", -2000560),                      // OST_RasterImages x7
    ("InitialViewSettings", -1),                    // None x4
    ("InteriorElev", -2000535),                     // OST_Elev x26
    ("InteriorElevArrow", -2000279),                // OST_Views x208
    ("InteriorElevAttributes", -1),                 // None x7
    ("JoistSystemAttr", -2001327),                  // OST_StructuralFramingSystem x4
    ("KeynoteTable", -1),                           // None x4
    ("KeynoteTagsOnSheetsTracker", -1),             // None x4
    ("KeynotingSystem", -1),                        // None x4
    ("LayoutNodesTracker", -1),                     // None x4
    ("LeaderStyle", -1),                            // None x888
    ("LegendComponent", -2000576),                  // OST_PreviewLegendComponents x518
    ("Level", -2000240),                            // OST_Levels x139
    ("LevelAttributes", -2000240),                  // OST_Levels x61
    ("LevelRoomPlan", -1),                          // None x2
    ("LightSchemeElement", -1),                     // None x29
    ("LinePatternElem", -1),                        // None x501
    ("ListConceptualSurfaceTypes", -1),             // None x4
    ("LoadCaseElem", -2005210),                     // OST_LoadCases x427
    ("LoadNatureElem", -1),                         // None x425
    ("MEPAnalyticalConnectionElementType", -2000983), // OST_AnalyticalPipeConnections x4
    ("MEPComponentTracker", -1),                    // None x4
    ("MEPHiddenLineSettingsElem", -1),              // None x4
    ("MEPNetworkDataElem", -1),                     // None x3
    ("MEPNetworkTracker", -1),                      // None x4
    ("MEPSystemTracker", -1),                       // None x4
    ("MEPSystemZoneElementType", -2001001),         // OST_MEPSystemZone x4
    ("MEPSystemZoneType", -2001001),                // OST_MEPSystemZone x3
    ("MaterialElem", -2000700),                     // OST_Materials x5204
    ("MechanicalEquipmentSetType", -2000985),       // OST_MechanicalEquipmentSet x4
    ("ModelClipBox", -2000301),                     // OST_SectionBox x26
    ("ModelGraphicsStyle", -1),                     // None x6
    ("MultiReferenceAnnotationType", -2000970),     // OST_MultiReferenceAnnotations x1
    ("MultipleValuesIndicationSettings", -1),       // None x4
    ("MultistoryStairsType", -1),                   // None x1
    ("NewCurtainWallType", -2000011),               // OST_Walls x8
    ("NumberingSchema", -1),                        // None x12
    ("Opening", -2000995),                          // OST_StructuralFramingOpening x4
    ("ParamBinding", -1),                           // None x2153
    ("ParamElemElectricalLoadClassification", -2008148), // OST_ParamElemElectricalLoadClassification x540
    ("ParamElemExternal", -1),                           // None x636
    ("ParamElemGlobal", -1),                             // None x1
    ("ParamElemProject", -1),                            // None x148
    ("ParameterFilterElement", -1),                      // None x152
    ("PathReinforcementType", -2009009),                 // OST_PathRein x1
    ("Pattern", -2000091),                               // OST_IOSArrays x4
    ("PenWidthTableElem", -1),                           // None x4
    ("PhaseFilterElem", -1),                             // None x27
    ("PipeFittingCenterLine", -2008049),                 // OST_PipeFitting x111
    ("PipeSegment", -2008163),                           // OST_PipeSegments x57
    ("PlanRegion", -2000191),                            // OST_PlanRegion x1
    ("PointCloudInstance", -2010001),                    // OST_PointClouds x10
    ("PointCloudType", -2010001),                        // OST_PointClouds x10
    ("PostedWarningElem", -1),                           // None x97
    ("PrintSettings", -1),                               // None x13
    ("ProfileRoof", -2000035),                           // OST_Roofs x7
    ("ProjectCopySettingsElem", -1),                     // None x4
    ("ProjectInfo", -2003101),                           // OST_ProjectInformation x53
    ("ProjectPhase", -2000112),                          // OST_Phases x8
    ("ProjectRevision", -2006070),                       // OST_Revisions x4
    ("PropertyAttr", -2001265),                          // OST_SiteProperty x6
    ("PropertySetLibrary", -1),                          // None x3
    ("RadialArray", -2000091),                           // OST_IOSArrays x1
    ("RampAttributes", -2000180),                        // OST_Ramps x55
    ("RbsCableTrayType", -2008130),                      // OST_CableTray x13
    ("RbsCableType", -2001119),                          // OST_Cable x3
    ("RbsConduitType", -2008132),                        // OST_Conduit x14
    ("RbsDbViewSystemNavigator", -1),                    // None x53
    ("RbsDistributionSysType", -2008041),                // OST_ElecDistributionSys x8
    ("RbsDuctInsulationType", -2008123),                 // OST_DuctInsulations x3
    ("RbsDuctLiningType", -2008124),                     // OST_DuctLinings x3
    ("RbsDuctSettingsElem", -1),                         // None x4
    ("RbsDuctSizesElem", -1),                            // None x4
    ("RbsFlexPipeType", -2008050),                       // OST_FlexPipeCurves x5
    ("RbsFluidType", -2008106),                          // OST_Fluids x76
    ("RbsHvacSystemType", -2008015),                     // OST_DuctSystem x12
    ("RbsPipeConnectionType", -2008103),                 // OST_PipeConnections x32
    ("RbsPipeCurve", -2008044),                          // OST_PipeCurves x160
    ("RbsPipeInsulationType", -2008122),                 // OST_PipeInsulations x4
    ("RbsPipeMaterialType", -2008104),                   // OST_PipeMaterials x265
    ("RbsPipeScheduleType", -2008105),                   // OST_PipeSchedules x50
    ("RbsPipeSettingsElem", -1),                         // None x4
    ("RbsPipeSizesElem", -1),                            // None x4
    ("RbsPipeType", -2008044),                           // OST_PipeCurves x59
    ("RbsPipingSystem", -2008043),                       // OST_PipingSystem x44
    ("RbsPipingSystemType", -2008043),                   // OST_PipingSystem x590
    ("RbsVoltageType", -2008040),                        // OST_ElectricalVoltage x12
    ("RbsWireInsulationType", -2008112),                 // OST_WireInsulations x107
    ("RbsWireMaterialType", -2008111),                   // OST_WireMaterials x16
    ("RbsWireSettingsElem", -1),                         // None x4
    ("RbsWireSizesElem", -1),                            // None x4
    ("RbsWireTemperatureRatingType", -2008113),          // OST_WireTemperatureRatings x12
    ("RbsWireType", -2008039),                           // OST_Wire x6
    ("ReactionsUpToDateElem", -1),                       // None x4
    ("Rebar", -2009000),                                 // OST_Rebar x314
    ("RebarBarType", -2009000),                          // OST_Rebar x19
    ("RebarBendingDetailType", -2001104),                // OST_RebarBendingDetails x8
    ("RebarHookType", -2009000),                         // OST_Rebar x10
    ("RebarInSystem", -2009000),                         // OST_Rebar x24
    ("RebarShape", -2009000),                            // OST_Rebar x36
    ("RebarSystem", -2009003),                           // OST_AreaRein x3
    ("RebarSystemType", -2009003),                       // OST_AreaRein x1
    ("ReconcileBrowserSettingsElem", -1),                // None x4
    ("RefPlane", -2000530),                              // OST_CLines x19
    ("ReferenceViewerAttributes", -1),                   // None x4
    ("RegenSplitterElem", -1),                           // None x103
    ("ReinforcementSettings", -1),                       // None x4
    ("RepeatingDetailAttributes", -2002000),             // OST_DetailComponents x15
    ("RevealAttr", -2000182),                            // OST_Reveals x5
    ("RevisionCloudAttr", -2006060),                     // OST_RevisionClouds x4
    ("RevisionCloudsOnSheetsTracker", -1),               // None x4
    ("RevisionNumberingSequence", -2006071),             // OST_RevisionNumberingSequences x8
    ("RoofAttributes", -2000035),                        // OST_Roofs x11
    ("RoofSoffitType", -2001393),                        // OST_RoofSoffit x54
    ("RoomElem", -2003200),                              // OST_Areas x4
    ("RouteAnalysisSettings", -1),                       // None x4
    ("RvtLinkInstance", -2001352),                       // OST_RvtLinks x3
    ("RvtLinkInstanceAppearanceParentElem", -1),         // None x4
    ("RvtLinkSymbol", -2001352),                         // OST_RvtLinks x3
    ("SFCostUserDataElem", -1),                          // None x1
    ("SSEPointVisibilitySettings", -1),                  // None x4
    ("STEPExportSettings", -1),                          // None x3
    ("SWall", -2000011),                                 // OST_Walls x193
    ("ScheduleInstanceBaseOnSheetsTracker", -1),         // None x4
    ("Section", -2000279),                               // OST_Views x62
    ("SectionAttributes", -1),                           // None x12
    ("SegmentCenterLine", -2008044),                     // OST_PipeCurves x160
    ("ShaftOpening", -2000996),                          // OST_ShaftOpening x13
    ("SheetsInSheetCollectionTracker", -1),              // None x3
    ("SketchGrid", -2000055),                            // OST_IOSSketchGrid x165
    ("SketchGridAppearanceParentElem", -1),              // None x4
    ("SketchPlane", -1),                                 // None x527
    ("SlaveSymbolTrackerElem", -1),                      // None x103
    ("SpanDirectionSymbol", -2005110),                   // OST_SpanDirectionSymbol x87
    ("SpotElevationStyle", -1),                          // None x617
    ("StackedWallType", -2003500),                       // OST_StackedWalls x5
    ("StairsAttributes", -2000120),                      // OST_Stairs x102
    ("StairsLandingType", -2000920),                     // OST_StairsLandings x13
    ("StairsPathType", -2000938),                        // OST_StairsPaths x8
    ("StairsRailingAttr", -2000126),                     // OST_StairsRailing x161
    ("StairsRunType", -2000919),                         // OST_StairsRuns x17
    ("StairsSupportType", -2000123),                     // OST_StairsStringerCarriage x10
    ("StairsType", -2000120),                            // OST_Stairs x169
    ("StructConnectionType", -1),                        // None x20
    ("StructSettingsElem", -1),                          // None x4
    ("StructuralConnectionSettings", -1),                // None x4
    ("SunAndShadowSettings", -2009609),                  // OST_SunStudy x365
    ("SunAnnotationElem", -2009609),                     // OST_SunStudy x150
    ("SweepHost", -2001391),                             // OST_Gutter x4
    ("SysMullionFamSym", -2000171),                      // OST_CurtainWallMullions x435
    ("SysPanelFamSym", -2000170),                        // OST_CurtainWallPanels x6
    ("TagNoteAttributes", -1),                           // None x54
    ("Text3dAttrSymbol", -2000151),                      // OST_GenericModel x5
    ("TextNote", -2000300),                              // OST_TextNotes x377
    ("TextNoteAttributes", -1),                          // None x459
    ("TilePatternType", -2003331),                       // OST_TilePatterns x68
    ("TopRailType", -2000946),                           // OST_RailingTopRail x6
    ("ToposolidType", -2001079),                         // OST_Toposolid x4
    ("TrueNorth", -1),                                   // None x4
    ("UnitsElem", -1),                                   // None x4
    ("VarSketch", -1),                                   // None x155
    ("ViewReferencing", -1),                             // None x4
    ("ViewSheetSet", -1),                                // None x6
    ("Viewer3d", -2000279),                              // OST_Views x29
    ("Viewport", -2000510),                              // OST_Viewports x80
    ("ViewportAttributes", -1),                          // None x15
    ("WallJoinDefaultSetting", -1),                      // None x4
    ("WatchElem", -1),                                   // None x12
    ("WorksetVisibilitySettingsElem", -1),               // None x4
    ("WorksharingViewModeSettings", -1),                 // None x5
    ("ZoneElement", -2008107),                           // OST_HVAC_Zones x3
    ("ZoneScheme", -1),                                  // None x4
];

/// The category every element of `class_name` reports, when the class decides it.
pub fn class_category(class_name: &str) -> Option<i64> {
    CLASS_CATEGORIES
        .iter()
        .find(|(class, _)| *class == class_name)
        .map(|(_, category)| *category)
}

/// Native classes whose elements, when they have a category, all expose the
/// IFC export parameters (`IFC_EXPORT_ELEMENT`, `IFC_GUID` on instances;
/// `IFC_EXPORT_ELEMENT_TYPE`, `IFC_TYPE_GUID` on types) in the API. Measured
/// on every file of the corpus; classes whose elements never or only
/// sometimes expose them (views, curves, groups, settings, ...) are absent.
const IFC_EXPORT_CLASSES: &[&str] = &[
    "AbsDuctType",               // x167
    "AbsFlexDuctType",           // x108
    "AreaReinforcementType",     // x1
    "BasicWallType",             // x1164
    "BuildingPadType",           // x4
    "CompoundCeilingType",       // x204
    "ContFootingType",           // x102
    "CorniceAttr",               // x55
    "CurtaSystemAttr",           // x54
    "CurtainRoofAttributes",     // x53
    "CutMarkType",               // x103
    "EdgeSlabAttr",              // x53
    "FabricSheetType",           // x1
    "FabricWireType",            // x1
    "FabricationPart",           // x16
    "FabricationPartCenterline", // x13
    "FabricationPartType",       // x102
    "FasciaAttr",                // x54
    "FilledRegion",              // x62
    "FilledRegionAttributes",    // x594
    "Floor",                     // x67
    "FloorAttributes",           // x342
    "Grid",                      // x10
    "GridAttributes",            // x158
    "GutterAttr",                // x53
    "HandRailType",              // x103
    "JoistSystemAttr",           // x53
    "Level",                     // x139
    "LevelAttributes",           // x61
    "MaterialElem",              // x5204
    "NewCurtainWallType",        // x152
    "PathReinforcementType",     // x1
    "PipeFittingCenterLine",     // x111
    "ProfileRoof",               // x7
    "PropertyAttr",              // x55
    "RampAttributes",            // x55
    "RbsCableTrayType",          // x207
    "RbsCableType",              // x28
    "RbsConduitType",            // x112
    "RbsDuctInsulationType",     // x3
    "RbsDuctLiningType",         // x3
    "RbsFlexPipeType",           // x54
    "RbsPipeCurve",              // x160
    "RbsPipeInsulationType",     // x4
    "RbsPipeType",               // x59
    "RbsPipingSystem",           // x44
    "RbsWireType",               // x55
    "Rebar",                     // x314
    "RebarBarType",              // x19
    "RebarInSystem",             // x24
    "RebarShape",                // x36
    "RebarSystem",               // x3
    "RebarSystemType",           // x1
    "RoofAttributes",            // x299
    "RoofSoffitType",            // x54
    "RoomElem",                  // x4
    "RvtLinkInstance",           // x3
    "RvtLinkSymbol",             // x3
    "SWall",                     // x193
    "SegmentCenterLine",         // x160
    "SharedSymbol",              // x2
    "StairsAttributes",          // x102
    "StairsLandingType",         // x160
    "StairsRailingAttr",         // x161
    "StairsRunType",             // x164
    "StairsSupportType",         // x108
    "StairsType",                // x169
    "SweepHost",                 // x4
    "SysMullionFamSym",          // x435
    "SysPanelFamSym",            // x151
    "Text3dAttrSymbol",          // x54
    "TopRailType",               // x151
    "ToposolidType",             // x53
    "ZoneElement",               // x39
];

/// Categories whose family symbols and instances expose the IFC export
/// parameters in the API (measured: every symbol and instance of these
/// categories in the corpus does, none of the other categories' do).
const FAMILY_IFC_EXPORT_CATEGORIES: &[i64] = &[
    -2009030, -2009013, -2008128, -2008126, -2008099, -2008087, -2008083, -2008075, -2008049,
    -2008016, -2008013, -2008010, -2003000, -2002000, -2001360, -2001350, -2001330, -2001320,
    -2001300, -2001180, -2001160, -2001140, -2001120, -2001060, -2001040, -2000949, -2000948,
    -2000151, -2000127, -2000120, -2000100, -2000080, -2000032, -2000023, -2000014, -2000011,
];

/// Whether every element of `class_name` with a category exposes the IFC
/// export parameters.
pub fn class_exposes_ifc_export(class_name: &str) -> bool {
    IFC_EXPORT_CLASSES.contains(&class_name)
}

/// Whether the family symbols and instances of `category` expose the IFC
/// export parameters.
pub fn family_category_exposes_ifc_export(category: i64) -> bool {
    FAMILY_IFC_EXPORT_CATEGORIES.contains(&category)
}
