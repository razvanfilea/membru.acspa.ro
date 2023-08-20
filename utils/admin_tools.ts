import {useEffect} from "react";
import {MemberTypes} from "../types/wrapper";
import {useRouter} from "next/router";
import useProfileData from "../hooks/useProfileData";

export function useExitIfNotFounder() {
    const profileData = useProfileData()
    const router = useRouter()

    useEffect(() => {
        console.log(profileData)
        if (!profileData.isLoading && (profileData.profile == null || profileData.profile.role !== MemberTypes.Fondator))
            router.back()
    }, [profileData, router])
}
