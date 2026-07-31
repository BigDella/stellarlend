import React, { useState, useCallback, useRef } from 'react';

interface SNSResolution {
  name: string;
  address: string;
}

interface SNSNameInputProps {
  value: string;
  onChange: (address: string, snsName?: string) => void;
  placeholder?: string;
  label?: string;
  disabled?: boolean;
}

export const SNSNameInput: React.FC<SNSNameInputProps> = ({
  value,
  onChange,
  placeholder = 'Enter address or SNS name (e.g. alice.stellar)',
  label,
  disabled = false,
}) => {
  const [inputValue, setInputValue] = useState(value);
  const [isResolving, setIsResolving] = useState(false);
  const [resolvedName, setResolvedName] = useState<SNSResolution | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [suggestions, setSuggestions] = useState<string[]>([]);
  const debounceRef = useRef<NodeJS.Timeout | null>(null);

  const isSNSName = (input: string): boolean => {
    return input.includes('.') || /^[a-zA-Z0-9_-]+$/.test(input);
  };

  const resolveSNS = useCallback(async (name: string) => {
    setIsResolving(true);
    setError(null);
    try {
      const response = await fetch(`/api/sns/validate?name=${encodeURIComponent(name)}`);
      const data = await response.json();
      if (data.success && data.data.valid) {
        const resolveResponse = await fetch(`/api/sns/resolve?name=${encodeURIComponent(name)}`);
        const resolveData = await resolveResponse.json();
        if (resolveData.success) {
          setResolvedName({ name, address: resolveData.data.address });
          onChange(resolveData.data.address, name);
        } else {
          setError(`SNS name "${name}" could not be resolved`);
          setResolvedName(null);
        }
      } else {
        setResolvedName(null);
      }
    } catch {
      setError('Failed to resolve SNS name');
      setResolvedName(null);
    }
    setIsResolving(false);
  }, [onChange]);

  const fetchSuggestions = useCallback(async (query: string) => {
    if (query.length < 2) {
      setSuggestions([]);
      return;
    }
    try {
      const response = await fetch(`/api/sns/names`);
      const data = await response.json();
      if (data.success) {
        const filtered = data.data.names
          .filter((name: string) => name.toLowerCase().includes(query.toLowerCase()))
          .slice(0, 5);
        setSuggestions(filtered);
      }
    } catch {
      setSuggestions([]);
    }
  }, []);

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newValue = e.target.value;
    setInputValue(newValue);
    setResolvedName(null);
    setError(null);

    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }

    if (isSNSName(newValue) && !newValue.startsWith('0x') && newValue.length > 2) {
      debounceRef.current = setTimeout(() => {
        fetchSuggestions(newValue);
        resolveSNS(newValue);
      }, 500);
    } else {
      setSuggestions([]);
      onChange(newValue);
    }
  };

  const handleSuggestionClick = (name: string) => {
    setInputValue(name);
    setSuggestions([]);
    resolveSNS(name);
  };

  const handleBlur = () => {
    if (inputValue && isSNSName(inputValue) && !inputValue.startsWith('0x')) {
      resolveSNS(inputValue);
    }
  };

  return (
    <div style={styles.container}>
      {label && <label style={styles.label}>{label}</label>}
      <div style={styles.inputWrapper}>
        <input
          type="text"
          value={inputValue}
          onChange={handleInputChange}
          onBlur={handleBlur}
          placeholder={placeholder}
          disabled={disabled}
          style={{
            ...styles.input,
            borderColor: resolvedName ? '#28a745' : error ? '#dc3545' : '#ddd',
          }}
        />
        {isResolving && <span style={styles.spinner}>...</span>}
        {resolvedName && (
          <div style={styles.resolvedBadge}>
            <span style={styles.checkmark}>✓</span>
            <span style={styles.resolvedName}>{resolvedName.name}</span>
          </div>
        )}
      </div>

      {suggestions.length > 0 && (
        <div style={styles.suggestions}>
          {suggestions.map((name) => (
            <div
              key={name}
              style={styles.suggestionItem}
              onClick={() => handleSuggestionClick(name)}
            >
              {name}
            </div>
          ))}
        </div>
      )}

      {error && <div style={styles.error}>{error}</div>}

      {resolvedName && (
        <div style={styles.resolvedInfo}>
          <span style={styles.resolvedLabel}>Resolved to:</span>
          <span style={styles.resolvedAddress}>{resolvedName.address}</span>
        </div>
      )}
    </div>
  );
};

const styles: Record<string, React.CSSProperties> = {
  container: {
    position: 'relative',
    marginBottom: '12px',
  },
  label: {
    display: 'block',
    fontSize: '12px',
    fontWeight: 'bold',
    color: '#666',
    marginBottom: '4px',
  },
  inputWrapper: {
    position: 'relative',
    display: 'flex',
    alignItems: 'center',
  },
  input: {
    width: '100%',
    padding: '10px 12px',
    borderRadius: '6px',
    border: '1px solid #ddd',
    fontSize: '14px',
    fontFamily: 'monospace',
    boxSizing: 'border-box',
    transition: 'border-color 0.2s',
  },
  spinner: {
    position: 'absolute',
    right: '12px',
    color: '#007bff',
    fontWeight: 'bold',
  },
  resolvedBadge: {
    position: 'absolute',
    right: '12px',
    display: 'flex',
    alignItems: 'center',
    gap: '4px',
    backgroundColor: '#d4edda',
    padding: '2px 8px',
    borderRadius: '12px',
    fontSize: '11px',
  },
  checkmark: {
    color: '#28a745',
    fontWeight: 'bold',
  },
  resolvedName: {
    color: '#155724',
    fontWeight: '600',
  },
  suggestions: {
    position: 'absolute',
    top: '100%',
    left: 0,
    right: 0,
    backgroundColor: 'white',
    border: '1px solid #ddd',
    borderRadius: '0 0 6px 6px',
    boxShadow: '0 4px 6px rgba(0,0,0,0.1)',
    zIndex: 1000,
    maxHeight: '150px',
    overflowY: 'auto',
  },
  suggestionItem: {
    padding: '8px 12px',
    cursor: 'pointer',
    fontSize: '13px',
    fontFamily: 'monospace',
    borderBottom: '1px solid #f0f0f0',
  },
  error: {
    marginTop: '4px',
    fontSize: '12px',
    color: '#dc3545',
  },
  resolvedInfo: {
    marginTop: '6px',
    padding: '6px 10px',
    backgroundColor: '#f8f9fa',
    borderRadius: '4px',
    fontSize: '11px',
    display: 'flex',
    gap: '8px',
    alignItems: 'center',
  },
  resolvedLabel: {
    color: '#666',
    fontWeight: '600',
  },
  resolvedAddress: {
    fontFamily: 'monospace',
    color: '#333',
    wordBreak: 'break-all',
  },
};
