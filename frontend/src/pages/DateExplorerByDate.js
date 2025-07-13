import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import NewSimpleDateSidebar from '../components/NewSimpleDateSidebar';
import SimpleDocumentsByDate from '../components/SimpleDocumentsByDate';
import './DateExplorer.css';

const DateExplorerByDate = () => {
  const [selectedDateKey, setSelectedDateKey] = useState(null);
  const navigate = useNavigate();

  const handleDateSelect = (dateKey) => {
    console.log('Date selected:', dateKey);
    setSelectedDateKey(dateKey);
  };

  const handleDocumentClick = (document) => {
    navigate(`/documents/${document.id}`);
  };

  return (
    <div style={{ display: 'flex', minHeight: 'calc(100vh - 80px)', marginTop: '80px' }}>
      <NewSimpleDateSidebar 
        onDocumentSelect={handleDateSelect}
        selectedDateKey={selectedDateKey}
      />
      
      <div style={{ flex: 1, marginLeft: '280px', background: 'white', minHeight: 'calc(100vh - 80px)', overflowY: 'auto' }}>
        <SimpleDocumentsByDate 
          dateKey={selectedDateKey}
          onDocumentClick={handleDocumentClick}
        />
      </div>
    </div>
  );
};

export default DateExplorerByDate;