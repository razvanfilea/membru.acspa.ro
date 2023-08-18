import React, {useCallback, useContext, useEffect, useState} from "react";
import {Profile} from "../../types/wrapper";
import {User, useSessionContext, useSupabaseClient} from "@supabase/auth-helpers-react";
import {Database} from "../../types/database.types";

export interface ProfileData {
    isLoading: boolean,
    profile: Profile | null
}

const defaultValue: ProfileData = {
    isLoading: true,
    profile: null,
}

const AuthContext = React.createContext(defaultValue)

export default function ProfileProvider({children}) {
    const supabase = useSupabaseClient<Database>()
    const sessionContext = useSessionContext()
    const [profileData, setProfileData] = useState<ProfileData>(defaultValue)

    useEffect(() => {
        async function getProfile(user: User) {
            try {
                setProfileData(defaultValue)

                let {data, error} = await supabase
                    .from('profiles')
                    .select('*')
                    .eq('id', user.id)
                    .single()

                if (data != null) {
                    setProfileData({isLoading: false, profile: data})
                } else {
                    console.error(error)
                }

            } catch (error) {
                alert('Error loading user data!')
                console.error(error)
                setProfileData({isLoading: false, profile: null})
            }
        }

        if (sessionContext.session != null) {
            setProfileData({isLoading: true, profile: null})
            getProfile(sessionContext.session!.user).then(() => console.log("Profile loaded successfully"))
        } else if (sessionContext.isLoading) {
            setProfileData({isLoading: true, profile: null})
        } else {
            console.log("Failed to get user session")
            setProfileData({isLoading: false, profile: null})
        }
    }, [sessionContext, supabase]);

    // Will be passed down to Signup, Login and Dashboard components
    const authDataCallback = useCallback((): ProfileData => {
        return profileData
    }, [profileData])

    return (
        <AuthContext.Provider value={authDataCallback()}>
            {children}
        </AuthContext.Provider>
    )
}

export function useProfile() {
    return useContext(AuthContext)
}
