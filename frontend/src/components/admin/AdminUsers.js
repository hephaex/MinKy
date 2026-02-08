import React from 'react';

const UserRow = ({ user, onUpdate }) => (
  <tr>
    <td>{user.username}</td>
    <td>{user.email}</td>
    <td>{user.full_name || 'N/A'}</td>
    <td>
      <span className={`status-badge ${user.is_active ? 'active' : 'inactive'}`}>
        {user.is_active ? 'Active' : 'Inactive'}
      </span>
    </td>
    <td>
      <span className={`admin-badge ${user.is_admin ? 'admin' : 'user'}`}>
        {user.is_admin ? 'Admin' : 'User'}
      </span>
    </td>
    <td>{user.document_count || 0}</td>
    <td>{user.comment_count || 0}</td>
    <td>
      <div className="user-actions">
        <button
          className="btn-small"
          onClick={() => onUpdate(user.id, { is_active: !user.is_active })}
        >
          {user.is_active ? 'Deactivate' : 'Activate'}
        </button>
        <button
          className="btn-small"
          onClick={() => onUpdate(user.id, { is_admin: !user.is_admin })}
        >
          {user.is_admin ? 'Remove Admin' : 'Make Admin'}
        </button>
      </div>
    </td>
  </tr>
);

const AdminUsers = ({ users, currentPage, onPageChange, onUserUpdate }) => {
  return (
    <div className="users-tab">
      <h2>User Management</h2>

      <div className="table-container">
        <table className="admin-table">
          <thead>
            <tr>
              <th>Username</th>
              <th>Email</th>
              <th>Full Name</th>
              <th>Status</th>
              <th>Role</th>
              <th>Documents</th>
              <th>Comments</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {users.map(user => (
              <UserRow
                key={user.id}
                user={user}
                onUpdate={onUserUpdate}
              />
            ))}
          </tbody>
        </table>
      </div>

      <div className="pagination">
        <button
          disabled={currentPage === 1}
          onClick={() => onPageChange(currentPage - 1)}
        >
          Previous
        </button>
        <span>Page {currentPage}</span>
        <button onClick={() => onPageChange(currentPage + 1)}>
          Next
        </button>
      </div>
    </div>
  );
};

export default AdminUsers;
