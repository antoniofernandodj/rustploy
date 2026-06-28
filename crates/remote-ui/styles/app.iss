/* Rustploy Remote — glacier-ui stylesheet.
   Terminal-flavoured dark theme, monospaced throughout, matching design/. */

/* ── Base ─────────────────────────────────────────────────────────────── */
.mono       { }
.screen     { width: fill; height: fill; background: #0A0E14; }

/* ── Sidebar ──────────────────────────────────────────────────────────── */
.sidebar        { width: 264; height: fill; background: #0D1117; padding: 28 20; spacing: 6; }
.brand_box      { spacing: 2; padding: 0 8 18 8; }
.brand          { size: 21; bold: true; color: #E6EDF3; }
.brand_sub      { size: 11; color: #6E7681; }
.nav_section    { size: 11; bold: true; color: #6E7681; padding: 16 8 6 8; }
.nav_item       { width: fill; color: #0D1117; padding: 11 14; }
.nav_item_on    { width: fill; color: #1F6FEB; padding: 11 14; }

/* ── Top bar ──────────────────────────────────────────────────────────── */
.topbar     { width: fill; padding: 18 28; spacing: 18; align-y: center; }
.search     { width: fill; padding: 11 14; }
.tab        { color: #0D1117; padding: 8 6; }
.tab_on     { color: #0D1117; padding: 8 6; }

/* ── Buttons ──────────────────────────────────────────────────────────── */
.btn_primary { color: #238636; padding: 11 20; }
.btn_blue    { color: #1F6FEB; padding: 11 22; }
.btn_danger  { color: #DA3633; padding: 11 20; }
.btn_ghost   { color: #21262D; padding: 11 18; }

/* ── Panels & cards ───────────────────────────────────────────────────── */
.content    { width: fill; height: fill; padding: 28 32; spacing: 22; background: #0A0E14; }
.panel      { background: #0D1117; border-radius: 14; border-width: 1; border-color: #21262D; padding: 4; }
.card       { background: #161B22; border-radius: 12; border-width: 1; border-color: #21262D; padding: 18; spacing: 10; }
.stat_card  { background: #0D1117; border-radius: 12; border-width: 1; border-color: #21262D; padding: 16 22; spacing: 6; }

/* ── Typography ───────────────────────────────────────────────────────── */
.title      { size: 30; bold: true; color: #E6EDF3; }
.subtitle   { size: 13; color: #8B949E; }
.muted      { size: 12; color: #8B949E; }
.label_cap  { size: 11; bold: true; color: #6E7681; }
.stat_num   { size: 26; bold: true; color: #58A6FF; }
.stat_num_g { size: 26; bold: true; color: #3FB950; }

/* ── Shell layout (composição, sem atributos inline no XML) ───────────── */
.main_col     { width: fill; height: fill; }
.nav_spacer   { height: fill; }
.header_row   { width: fill; align-y: start; }
.header_titles{ width: fill; spacing: 6; }
.stats_row    { spacing: 14; }
.scroll_fill  { width: fill; height: fill; }
.table_body   { width: fill; }
.row_wrap     { width: fill; }

/* ── Table ────────────────────────────────────────────────────────────── */
.thead      { width: fill; padding: 14 18; background: #161B22; border-radius: 10; spacing: 0; }
.th         { size: 11; bold: true; color: #6E7681; }
.trow       { width: fill; padding: 13 18; spacing: 0; align-y: center; }
.td         { size: 13; color: #C9D1D9; }
.td_key     { size: 13; bold: true; color: #58A6FF; }
.col_svc    { width: 240; }
.col_proj   { width: 180; }
.col_state  { width: 170; }
.col_dur    { width: 150; }
.col_start  { width: fill; }
.content_center { width: fill; height: fill; align-x: center; align-y: center; }
.panel_fill { width: fill; height: fill; background: #0D1117; border-radius: 14; border-width: 1; border-color: #21262D; padding: 4; }
.state_cell { width: 170; spacing: 7; align-y: center; }
.state_dot  { size: 11; }
.state_lbl  { size: 12; }

/* ── Login ────────────────────────────────────────────────────────────── */
.login_wrap   { width: fill; height: fill; align-x: center; align-y: center; background: #0A0E14; }
.login_card   { width: 460; align-x: start; background: #0D1117; border-radius: 16; border-width: 1; border-color: #21262D; padding: 32; spacing: 16; }
.login_head   { width: fill; spacing: 14; align-y: center; }
.login_brand  { width: fill; spacing: 6; align-x: start; }
.field_group  { width: fill; spacing: 6; }
.field_row    { width: fill; }
.login_checks { width: fill; spacing: 8; padding: 4 0; }
.status_row   { spacing: 14; align-y: center; }
.login_title  { size: 30; bold: true; color: #A9C7FF; }
.field_label  { size: 11; bold: true; color: #8B949E; }
.field_label_fill { width: fill; size: 11; bold: true; color: #8B949E; }
.field_input  { width: fill; padding: 13 14; }
.connect_btn  { width: fill; color: #2F81F7; padding: 14; text-align: center; }

/* ── Status badge text colors (used inline on the dot/label) ──────────── */
.ok    { color: #3FB950; }
.warn  { color: #D29922; }
.err   { color: #F85149; }
