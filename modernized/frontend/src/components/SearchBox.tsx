/**
 * Search box component for ticket queue.
 * Quick search text input with Advanced link to open filter dialog.
 *
 * @implements TS-M3-F3: search box + advanced search dialog UI
 */
import { useState, type KeyboardEvent } from "react";
import { observer } from "mobx-react-lite";
import {
  Alert,
  Box,
  Button,
  IconButton,
  InputAdornment,
  Stack,
  TextField,
  Tooltip,
} from "@mui/material";
import { Clear, Search, Tune } from "@mui/icons-material";
import { useStores } from "../stores/StoreContext";

export interface SearchBoxProps {
  /** Current search query (from URL). */
  query?: string;
  /** Called when search is submitted. */
  onSearch: (query: string) => void;
  /** Called to open advanced search dialog. */
  onAdvancedOpen: () => void;
  /** Called to clear search. */
  onClear: () => void;
  /** True if search is active. */
  isSearchActive?: boolean;
}

/**
 * Quick search text box with Advanced link.
 * @implements TS-M3-F3 AC-1: Search box visible above listing
 * @implements TS-M3-F3 AC-2: Quick search submits keyword
 * @implements TS-M3-F3 AC-3: Search < 3 chars shows error
 */
export const SearchBox = observer(function SearchBox({
  query = "",
  onSearch,
  onAdvancedOpen,
  onClear,
  isSearchActive = false,
}: SearchBoxProps) {
  const { staffTickets } = useStores();
  const [inputValue, setInputValue] = useState(query);

  const handleSubmit = () => {
    const trimmed = inputValue.trim();
    if (staffTickets.validateSearchQuery(trimmed)) {
      onSearch(trimmed);
    }
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Enter") {
      handleSubmit();
    }
  };

  const handleClear = () => {
    setInputValue("");
    staffTickets.setSearchMode(false);
    onClear();
  };

  return (
    <Box data-testid="search-box">
      <Stack direction="row" spacing={1} alignItems="flex-start">
        <TextField
          size="small"
          placeholder="Search tickets..."
          value={inputValue}
          onChange={(e) => setInputValue(e.target.value)}
          onKeyDown={handleKeyDown}
          error={Boolean(staffTickets.searchError)}
          helperText={staffTickets.searchError}
          InputProps={{
            startAdornment: (
              <InputAdornment position="start">
                <Search fontSize="small" />
              </InputAdornment>
            ),
            endAdornment: inputValue ? (
              <InputAdornment position="end">
                <IconButton
                  size="small"
                  onClick={() => setInputValue("")}
                  aria-label="Clear search input"
                >
                  <Clear fontSize="small" />
                </IconButton>
              </InputAdornment>
            ) : null,
          }}
          sx={{ minWidth: 280 }}
          inputProps={{ "data-testid": "search-input" }}
        />
        <Button
          variant="contained"
          onClick={handleSubmit}
          startIcon={<Search />}
          data-testid="search-submit"
        >
          Search
        </Button>
        <Tooltip title="Advanced Search">
          <Button
            variant="outlined"
            onClick={onAdvancedOpen}
            startIcon={<Tune />}
            data-testid="search-advanced-button"
          >
            Advanced
          </Button>
        </Tooltip>
        {isSearchActive && (
          <Button
            variant="text"
            color="secondary"
            onClick={handleClear}
            startIcon={<Clear />}
            data-testid="search-clear"
          >
            Clear Search
          </Button>
        )}
      </Stack>
      {isSearchActive && !staffTickets.searchError && (
        <Alert severity="info" sx={{ mt: 1 }} data-testid="search-results-banner">
          Showing search results{query ? ` for "${query}"` : ""}
        </Alert>
      )}
    </Box>
  );
});
