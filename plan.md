# AxKeyStore

## Flow

1. AxKeyStore is an open source command line tool which stores keys and passwords securely in the user's GitHub repo.

2. User should setup a private GitHub repo to store the keys and passwords.

3. AxKeyStore authenticates user using their GitHub OAUTH credentials. The token is stored in the user's local machine in encrypted form using master key.

4. Once the authentication is through, user should give write access to the already setup private GitHub repo to AxKeyStore application. Repo name is stored locally in encrypted form.

5. Once the application receives the write access, the application is ready to store the keys and passwords.

6. User can use the axkeystore command to store the keys and passwords.

7. User can add key values, get key values, update key values and delete key values.

8. User can also add categories to organize the keys and passwords.

9. if no value is given for a key, generate a alpha numeric value with only alphabets. Max length shall be 36 characters. Minimum shall be 6. Then show that value to the user and ask for confirmation.

10. User shall be able to list all versions of a key. List 10 versions at a time. User can ask for more versions.

11. User shall be able to get the value of a previous version of a key.

### Master Password management

1. User can set a master password for the application.

2. User cannot remove the master password for the application.

3. If master password is not set, user shall be asked to set it.

4. If master password is set, user shall be asked to enter it before storing or retrieving any key.

5. If master password is set, user shall be asked to enter it before updating or deleting any key.

6. The master password is used to encrypt a 36 character long random string. This encrypted string is called master key.

7. Master key shall be stored in github private repo in encrypted form.

8. The master key shall be used to encrypt the actual key values.

### Master Password for local master key

1. 36 character long random string is generated. This is called as local master key

2. Local master key is stored in the user's local machine in encrypted form using master password.

3. Local master key is used to encrypt auth credentials (refresh token and repo name) for each profile.

### Reset Master Password

1. User shall be able to reset their password.

2. User shall be asked to enter the old password.

3. User shall be asked to enter the new password.

4. User shall be asked to confirm the new password.

5. If the new password and confirmation password match, the password shall be updated.

6. If the new password and confirmation password do not match, the password shall not be updated.

7. If the old password is incorrect, the password shall not be updated.

8. local and remote master keys shall be decrypted using old password and encrypted using new password.

9. the new encrypted local key shall be saved to local config file.

10. the new encrypted remote key shall be saved to remote config file.

11. remote key will be saved first. Ony on successful updation of remote key, local key shall be updated.

12. In case of any failure, the old password shall be used to decrypt the local and remote master keys.

### Profile Management

1. User can create multiple profiles.
2. Each profile will have its own login, master password and github repo.
3. User can switch between profiles.
4. User can delete profiles.
5. User can list all profiles.
6. User can set a profile when running the init, store, get, history or delete commands.
7. If no profile is provided along with the command, it will use the directory axkeystore.
8. If profile is provided along with the command, it will use the directory axkeystore/<profile_name>.
9. Profile name shall contain only alphabets and numbers. No spaces or special characters except '\_' and '-'.
