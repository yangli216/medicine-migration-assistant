import {
  Fragment,
  startTransition,
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
} from "react";
import { CaretDown, Check, MagnifyingGlass } from "@phosphor-icons/react";
import { createPortal } from "react-dom";

function normalizeOption(option) {
  if (typeof option === "string") {
    return {
      value: option,
      label: option,
      keywords: option,
      description: "",
      meta: "",
      group: "",
      data: undefined,
    };
  }
  return {
    value: `${option.value ?? ""}`,
    label: `${option.label ?? option.value ?? ""}`,
    keywords: `${option.keywords ?? ""}`,
    description: `${option.description ?? ""}`,
    meta: `${option.meta ?? ""}`,
    group: `${option.group ?? ""}`,
    disabled: Boolean(option.disabled),
    data: option.data,
  };
}

export function SearchableSelect({
  value,
  onChange,
  options,
  placeholder = "请选择",
  searchPlaceholder = "输入关键词过滤",
  ariaLabel = "下拉选择",
  emptyText = "没有匹配项",
  allowCustom = false,
  disabled = false,
  className = "",
  maxVisibleOptions = 200,
  loadOptions,
  minimumSearchLength = 1,
  loadingText = "正在搜索…",
}) {
  const rawId = useId();
  const listboxId = `searchable-select-${rawId.replace(/:/g, "")}`;
  const rootRef = useRef(null);
  const menuRef = useRef(null);
  const searchRef = useRef(null);
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const [menuStyle, setMenuStyle] = useState({});
  const [remoteOptions, setRemoteOptions] = useState([]);
  const [remoteLoading, setRemoteLoading] = useState(false);
  const loadOptionsRef = useRef(loadOptions);
  loadOptionsRef.current = loadOptions;

  const mergedOptions = useMemo(() => {
    const seen = new Set();
    return [...options, ...remoteOptions].filter((option) => {
      const key = `${typeof option === "string" ? option : option.value ?? ""}`;
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
  }, [options, remoteOptions]);
  const normalizedOptions = useMemo(
    () => mergedOptions.map(normalizeOption),
    [mergedOptions],
  );
  const selected = normalizedOptions.find(
    (option) => option.value === `${value ?? ""}`,
  );
  const selectedLabel = selected?.label || `${value ?? ""}`;
  const normalizedQuery = query.trim().toLocaleLowerCase("zh-CN");
  const filteredOptions = normalizedOptions.filter((option) =>
    `${option.label} ${option.value} ${option.keywords} ${option.description} ${option.meta}`
      .toLocaleLowerCase("zh-CN")
      .includes(normalizedQuery),
  );
  const customValue = query.trim();
  const canUseCustom =
    allowCustom &&
    customValue &&
    !normalizedOptions.some(
      (option) =>
        option.value.toLocaleLowerCase("zh-CN") ===
          customValue.toLocaleLowerCase("zh-CN") ||
        option.label.toLocaleLowerCase("zh-CN") ===
          customValue.toLocaleLowerCase("zh-CN"),
    );
  const displayedOptions = canUseCustom
    ? [
        ...filteredOptions,
        {
          value: customValue,
          label: `使用“${customValue}”`,
          keywords: customValue,
          custom: true,
        },
      ]
    : filteredOptions;
  const visibleOptions = displayedOptions.slice(0, maxVisibleOptions);

  const updateMenuPosition = () => {
    const trigger = rootRef.current;
    if (!trigger) return;
    const rect = trigger.getBoundingClientRect();
    const estimatedHeight = Math.min(330, 86 + displayedOptions.length * 42);
    const roomBelow = window.innerHeight - rect.bottom;
    const openUpward = roomBelow < estimatedHeight && rect.top > roomBelow;
    setMenuStyle({
      left: rect.left,
      width: Math.max(rect.width, 220),
      top: openUpward ? "auto" : rect.bottom + 6,
      bottom: openUpward ? window.innerHeight - rect.top + 6 : "auto",
    });
  };

  useEffect(() => {
    if (!open) return undefined;
    setQuery("");
    setActiveIndex(
      Math.max(
        0,
        normalizedOptions.findIndex(
          (option) => option.value === `${value ?? ""}`,
        ),
      ),
    );
    updateMenuPosition();
    const focusTimer = window.requestAnimationFrame(() =>
      searchRef.current?.focus(),
    );
    const closeWhenClickingOutside = (event) => {
      if (
        !rootRef.current?.contains(event.target) &&
        !menuRef.current?.contains(event.target)
      ) {
        setOpen(false);
      }
    };
    window.addEventListener("resize", updateMenuPosition);
    window.addEventListener("scroll", updateMenuPosition, true);
    document.addEventListener("pointerdown", closeWhenClickingOutside);
    return () => {
      window.cancelAnimationFrame(focusTimer);
      window.removeEventListener("resize", updateMenuPosition);
      window.removeEventListener("scroll", updateMenuPosition, true);
      document.removeEventListener("pointerdown", closeWhenClickingOutside);
    };
  }, [open]);

  useEffect(() => {
    if (!open) return;
    setActiveIndex(0);
    updateMenuPosition();
  }, [query]);

  useEffect(() => {
    const remoteLoader = loadOptionsRef.current;
    const normalizedSearch = query.trim();
    if (
      !open ||
      !remoteLoader ||
      normalizedSearch.length < minimumSearchLength
    ) {
      setRemoteOptions([]);
      setRemoteLoading(false);
      return undefined;
    }
    let active = true;
    const timer = window.setTimeout(async () => {
      setRemoteLoading(true);
      try {
        const loaded = await remoteLoader(normalizedSearch);
        if (active) setRemoteOptions(Array.isArray(loaded) ? loaded : []);
      } catch {
        if (active) setRemoteOptions([]);
      } finally {
        if (active) setRemoteLoading(false);
      }
    }, 220);
    return () => {
      active = false;
      window.clearTimeout(timer);
    };
  }, [open, query, minimumSearchLength]);

  useEffect(() => {
    if (!open) return;
    menuRef.current
      ?.querySelector('[data-active="true"]')
      ?.scrollIntoView({ block: "nearest" });
  }, [activeIndex, open]);

  const choose = (option) => {
    if (option.disabled) return;
    setOpen(false);
    setQuery("");
    window.requestAnimationFrame(() =>
      rootRef.current?.querySelector("button")?.focus(),
    );
    startTransition(() => onChange(option.value, option));
  };

  const handleKeyboard = (event) => {
    if (["ArrowDown", "ArrowUp"].includes(event.key)) {
      event.preventDefault();
      if (!open) {
        setOpen(true);
        return;
      }
      if (!visibleOptions.length) return;
      const direction = event.key === "ArrowDown" ? 1 : -1;
      setActiveIndex(
        (current) =>
          (current + direction + visibleOptions.length) % visibleOptions.length,
      );
      return;
    }
    if (event.key === "Enter" && open && visibleOptions[activeIndex]) {
      event.preventDefault();
      choose(visibleOptions[activeIndex]);
      return;
    }
    if (event.key === "Escape" && open) {
      event.preventDefault();
      setOpen(false);
    }
  };

  return (
    <div
      className={`searchable-select ${open ? "searchable-select--open" : ""} ${className}`}
      ref={rootRef}
    >
      <button
        aria-controls={listboxId}
        aria-expanded={open}
        aria-haspopup="listbox"
        aria-label={ariaLabel}
        className="searchable-select__trigger"
        disabled={disabled}
        onClick={() => setOpen((current) => !current)}
        onKeyDown={handleKeyboard}
        role="combobox"
        type="button"
      >
        <span className={selectedLabel ? "" : "is-placeholder"}>
          {selectedLabel || placeholder}
        </span>
        <CaretDown size={16} weight="bold" />
      </button>
      {open &&
        createPortal(
          <div
            className="searchable-select__menu"
            ref={menuRef}
            style={menuStyle}
          >
            <div className="searchable-select__search">
              <MagnifyingGlass size={16} />
              <input
                aria-label={`过滤${ariaLabel}`}
                onChange={(event) => setQuery(event.target.value)}
                onKeyDown={handleKeyboard}
                placeholder={searchPlaceholder}
                ref={searchRef}
                type="search"
                value={query}
              />
            </div>
            <div
              className="searchable-select__options"
              id={listboxId}
              role="listbox"
            >
              {visibleOptions.map((option, index) => (
                <Fragment
                  key={`${option.custom ? "custom" : "option"}-${option.value}`}
                >
                  {option.group &&
                    option.group !== visibleOptions[index - 1]?.group && (
                      <div
                        className="searchable-select__group"
                        role="presentation"
                      >
                        {option.group}
                      </div>
                    )}
                  <button
                    aria-disabled={option.disabled}
                    aria-selected={option.value === `${value ?? ""}`}
                    className={`searchable-select__option ${option.custom ? "searchable-select__option--custom" : ""} ${option.disabled ? "is-disabled" : ""}`}
                    data-active={activeIndex === index}
                    disabled={option.disabled}
                    onClick={() => choose(option)}
                    onMouseEnter={() => setActiveIndex(index)}
                    role="option"
                    type="button"
                  >
                    <span className="searchable-select__option-copy">
                      <strong>{option.label}</strong>
                      {option.description && (
                        <small>{option.description}</small>
                      )}
                      {option.meta && <em>{option.meta}</em>}
                    </span>
                    {option.value === `${value ?? ""}` && (
                      <Check size={16} weight="bold" />
                    )}
                  </button>
                </Fragment>
              ))}
              {remoteLoading && (
                <div className="searchable-select__empty" role="status">
                  {loadingText}
                </div>
              )}
              {!remoteLoading && !displayedOptions.length && (
                <div className="searchable-select__empty">{emptyText}</div>
              )}
              {displayedOptions.length > visibleOptions.length && (
                <div className="searchable-select__more">
                  还有 {displayedOptions.length - visibleOptions.length}{" "}
                  项，请继续输入关键词缩小范围
                </div>
              )}
            </div>
          </div>,
          document.body,
        )}
    </div>
  );
}
