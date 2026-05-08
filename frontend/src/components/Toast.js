import PropTypes from 'prop-types';
import './Toast.css';

const Toast = ({ message, type = 'success', onDismiss }) => {
  if (!message) return null;

  return (
    <div className={`toast toast--${type}`} role="alert">
      <span className="toast-message">{message}</span>
      {onDismiss && (
        <button className="toast-dismiss" onClick={onDismiss} aria-label="Dismiss">
          &times;
        </button>
      )}
    </div>
  );
};

Toast.propTypes = {
  message: PropTypes.string,
  type: PropTypes.oneOf(['success', 'error', 'info']),
  onDismiss: PropTypes.func,
};

export default Toast;
